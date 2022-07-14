use itertools::izip;

use crate::error::CompactEcashError;
use crate::scheme::{PartialWallet, Payment, pseudorandom_fgt};
use crate::scheme::aggregation::{
    aggregate_signature_shares, aggregate_verification_keys, aggregate_wallets,
};
use crate::scheme::identify::identify;
use crate::scheme::keygen::{
    generate_keypair_user, PublicKeyUser, SecretKeyUser, ttp_keygen, VerificationKeyAuth,
};
use crate::scheme::PayInfo;
use crate::scheme::setup::{GroupParameters, Parameters, setup};
use crate::scheme::withdrawal::{issue_verify, issue_wallet, withdrawal_request};
use crate::utils::{hash_to_scalar, SignatureShare};

#[test]
fn main() -> Result<(), CompactEcashError> {
    let L = 32;
    let params = setup(L);
    let grparams = params.grp();
    let user_keypair = generate_keypair_user(&grparams);

    let (req, req_info) = withdrawal_request(grparams, &user_keypair.secret_key()).unwrap();
    let authorities_keypairs = ttp_keygen(&grparams, 2, 3).unwrap();

    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3]))?;

    let mut wallet_blinded_signatures = Vec::new();
    for auth_keypair in authorities_keypairs {
        let blind_signature = issue_wallet(
            &grparams,
            auth_keypair.secret_key(),
            user_keypair.public_key(),
            &req,
        );
        wallet_blinded_signatures.push(blind_signature.unwrap());
    }

    let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
        wallet_blinded_signatures.iter(),
        verification_keys_auth.iter()
    )
        .map(|(w, vk)| issue_verify(&grparams, vk, &user_keypair.secret_key(), w, &req_info).unwrap())
        .collect();

    // Aggregate partial wallets
    let aggr_wallet = aggregate_wallets(
        &grparams,
        &verification_key,
        &user_keypair.secret_key(),
        &unblinded_wallet_shares,
        &req_info,
    )?;

    // Let's try to spend some coins
    let pay_info = PayInfo { info: [6u8; 32] };
    let spend_vv = 1;

    let (payment, upd_wallet) = aggr_wallet.spend(
        &params,
        &verification_key,
        &user_keypair.secret_key(),
        &pay_info,
        false,
        spend_vv,
    )?;

    assert!(payment
        .spend_verify(&params, &verification_key, &pay_info, spend_vv)
        .unwrap());

    // try to spend twice the same payment with different payInfo
    let payment1 = payment.clone();
    let pay_info2 = PayInfo { info: [9u8; 32] };
    let rr2 = hash_to_scalar(pay_info2.info);
    let l2 = aggr_wallet.l() - 1;
    let payment2 = Payment {
        kappa: payment1.kappa.clone(),
        sig: payment1.sig.clone(),
        ss: payment1.ss.clone(),
        tt: vec![grparams.gen1() * user_keypair.secret_key().sk
            + pseudorandom_fgt(&grparams, aggr_wallet.t(), l2) * rr2],
        aa: payment1.aa.clone(),
        cc: payment1.cc.clone(),
        dd: payment1.dd.clone(),
        rr: vec![rr2],
        kappa_k: payment1.kappa_k.clone(),
        sig_lk: payment1.sig_lk.clone(),
        zk_proof: payment1.zk_proof.clone(),
        vv: spend_vv,
    };

    let identified_user = identify(payment1, payment2, pay_info, pay_info2).unwrap();
    // assert_eq!(user_keypair.public_key().pk, identified_user.pk);

    Ok(())
}
