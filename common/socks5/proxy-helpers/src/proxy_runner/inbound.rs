// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::Chunker;
use super::MixProxySender;
use super::SHUTDOWN_TIMEOUT;
use crate::available_reader::AvailableReader;
use bytes::Bytes;
use client_core::client::real_messages_control::acknowledgement_control::input_message_listener::FreshInputMessageChunker;
use futures::FutureExt;
use futures::StreamExt;
use log::*;
use ordered_buffer::OrderedMessageSender;
use rand::rngs::OsRng;
use socks5_requests::ConnectionId;
use std::time::Duration;
use std::{io, sync::Arc};
use task::ShutdownListener;
use tokio::select;
use tokio::{net::tcp::OwnedReadHalf, sync::Notify, time::sleep};

async fn send_empty_close<F, S>(
    connection_id: ConnectionId,
    message_sender: &mut OrderedMessageSender,
    mix_sender: &MixProxySender<S>,
    adapter_fn: F,
) where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S,
{
    let ordered_msg = message_sender.wrap_message(Vec::new()).into_bytes();
    if mix_sender
        .send(adapter_fn(connection_id, ordered_msg, true))
        .await
        .is_err()
    {
        panic!();
    };
}

async fn deal_with_data<F, S>(
    read_data: Option<io::Result<Bytes>>,
    local_destination_address: &str,
    remote_source_address: &str,
    connection_id: ConnectionId,
    message_sender: &mut OrderedMessageSender,
    mix_sender: &MixProxySender<S>,
    adapter_fn: F,
    msg_chunker: &mut Option<Box<dyn Chunker<S>>>,
) -> bool
where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S,
{
    let (read_data, is_finished) = match read_data {
        Some(data) => match data {
            Ok(data) => (data, false),
            Err(err) => {
                error!(target: &*format!("({}) socks5 inbound", connection_id), "failed to read request from the socket - {}", err);
                (Default::default(), true)
            }
        },
        None => (Default::default(), true),
    };

    debug!(
        target: &*format!("({}) socks5 inbound", connection_id),
        "[{} bytes]\t{} → local → mixnet → remote → {}. Local closed: {}",
        read_data.len(),
        local_destination_address,
        remote_source_address,
        is_finished
    );

    // if we're sending through the mixnet increase the sequence number...
    let ordered_msg = message_sender.wrap_message(read_data.to_vec()).into_bytes();

    // WIP(JON): here we do the chunking, and send to real_message_sender instead
    if let Some(chunker) = msg_chunker {
        log::info!("({connection_id}): chunking and sending");
        let msg = adapter_fn(connection_id, ordered_msg, is_finished);
        chunker.chunk(msg).await;
        log::info!("({connection_id}): chunking and sending done");
    } else {
        log::info!("ordered_msg.len: {}", ordered_msg.len());
        if ordered_msg.len() > 10000 {
            log::info!("Sending large");
            log::info!("capacity: {}", mix_sender.capacity());
            loop {
                if mix_sender.capacity() > 2 {
                    if mix_sender
                        .send(adapter_fn(connection_id, ordered_msg, is_finished))
                        .await
                        .is_err()
                    {
                        panic!();
                    }
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        } else {
            log::info!("Sending smaller");
            if mix_sender
                .send(adapter_fn(connection_id, ordered_msg, is_finished))
                .await
                .is_err()
            {
                panic!();
            }
        }
    }

    if is_finished {
        // technically we already informed it when we sent the message to mixnet above
        debug!(target: &*format!("({}) socks5 inbound", connection_id), "The local socket is closed - won't receive any more data. Informing remote about that...");
    }

    is_finished
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_inbound<F, S>(
    mut reader: OwnedReadHalf,
    local_destination_address: String, // addresses are provided for better logging
    remote_source_address: String,
    connection_id: ConnectionId,
    mix_sender: MixProxySender<S>,
    adapter_fn: F,
    shutdown_notify: Arc<Notify>,
    mut shutdown_listener: ShutdownListener,
    mut msg_chunker: Option<Box<dyn Chunker<S>>>,
) -> OwnedReadHalf
where
    F: Fn(ConnectionId, Vec<u8>, bool) -> S + Send + 'static,
{
    let mut available_reader = AvailableReader::new(&mut reader);
    let mut message_sender = OrderedMessageSender::new();
    let shutdown_future = shutdown_notify.notified().then(|_| sleep(SHUTDOWN_TIMEOUT));

    tokio::pin!(shutdown_future);

    loop {
        select! {
            read_data = &mut available_reader.next() => {
                if deal_with_data(read_data, &local_destination_address, &remote_source_address, connection_id, &mut message_sender, &mix_sender, &adapter_fn, &mut msg_chunker).await {
                    break
                }
            }
            _ = &mut shutdown_future => {
                debug!("closing inbound proxy after outbound was closed {:?} ago", SHUTDOWN_TIMEOUT);
                // inform remote just in case it was closed because of lack of heartbeat.
                // worst case the remote will just have couple of false negatives
                send_empty_close(connection_id, &mut message_sender, &mix_sender, &adapter_fn).await;
                break;
            }
            _ = shutdown_listener.recv() => {
                log::trace!("ProxyRunner inbound: Received shutdown");
                break;
            }
        }
    }
    trace!("{} - inbound closed", connection_id);
    shutdown_notify.notify_one();

    reader
}
