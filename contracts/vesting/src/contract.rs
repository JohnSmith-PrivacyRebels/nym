use crate::errors::ContractError;
use crate::queued_migrations::migrate_config_from_env;
use crate::storage::{
    account_from_address, locked_pledge_cap, update_locked_pledge_cap, BlockTimestampSecs, NodeId,
    ADMIN, DELEGATIONS, MIXNET_CONTRACT_ADDRESS, MIX_DENOM, OLD_DELEGATIONS,
};
use crate::traits::{
    DelegatingAccount, GatewayBondingAccount, MixnodeBondingAccount, VestingAccount,
};
use crate::vesting::{populate_vesting_periods, Account};
use cosmwasm_std::{
    coin, entry_point, to_binary, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult, Timestamp, Uint128,
};
use cw_storage_plus::Bound;
use mixnet_contract_common::{Gateway, IdentityKey, MixNode};
use vesting_contract_common::events::{
    new_ownership_transfer_event, new_periodic_vesting_account_event,
    new_staking_address_update_event, new_track_gateway_unbond_event,
    new_track_mixnode_unbond_event, new_track_reward_event, new_track_undelegation_event,
    new_vested_coins_withdraw_event,
};
use vesting_contract_common::messages::{
    ExecuteMsg, InitMsg, MigrateMsg, QueryMsg, VestingSpecification,
};
use vesting_contract_common::{
    AllDelegationsResponse, DelegationTimesResponse, OriginalVestingResponse, Period, PledgeData,
    VestingDelegation,
};

pub const INITIAL_LOCKED_PLEDGE_CAP: Uint128 = Uint128::new(100_000_000_000);

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    // ADMIN is set to the address that instantiated the contract, TODO: make this updatable
    ADMIN.save(deps.storage, &info.sender.to_string())?;
    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &msg.mixnet_contract_address)?;
    MIX_DENOM.save(deps.storage, &msg.mix_denom)?;
    Ok(Response::default())
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    migrate_config_from_env(_deps, _env, _msg)?;
    Ok(Response::default())
}

fn update_delegation_to_v2(
    deps: DepsMut<'_>,
    info: MessageInfo,
    owner: String,
    node_identity: String,
    mix_id: NodeId,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }

    // this MUST succeed since we know this delegation was created via vesting contract...
    let account = account_from_address(&owner, deps.storage, deps.api)?;
    let storage_prefix = (account.storage_key(), node_identity);
    let old_data = OLD_DELEGATIONS
        .prefix(storage_prefix.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (timestamp, amount) in old_data {
        OLD_DELEGATIONS.remove(
            deps.storage,
            (storage_prefix.0, storage_prefix.1.clone(), timestamp),
        );
        DELEGATIONS.save(deps.storage, (storage_prefix.0, mix_id, timestamp), &amount)?;
    }

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AuthorisedUpdateToV2 {
            owner,
            node_identity,
            mix_id,
        } => update_delegation_to_v2(deps, info, owner, node_identity, mix_id),
        ExecuteMsg::TrackUndelegation {
            owner,
            mix_identity,
            amount,
        } => try_track_undelegation(&owner, mix_identity, amount, info, deps),
        _ => Err(ContractError::MaintenanceMode),
        // ExecuteMsg::UpdateLockedPledgeCap { amount } => {
        //     try_update_locked_pledge_cap(amount, info, deps)
        // }
        // ExecuteMsg::TrackReward { amount, address } => {
        //     try_track_reward(deps, info, amount, &address)
        // }
        // ExecuteMsg::ClaimOperatorReward {} => try_claim_operator_reward(deps, info),
        // ExecuteMsg::ClaimDelegatorReward { mix_identity } => {
        //     try_claim_delegator_reward(deps, info, mix_identity)
        // }
        // ExecuteMsg::CompoundDelegatorReward { mix_identity } => {
        //     try_compound_delegator_reward(mix_identity, info, deps)
        // }
        // ExecuteMsg::CompoundOperatorReward {} => try_compound_operator_reward(info, deps),
        // ExecuteMsg::UpdateMixnodeConfig {
        //     profit_margin_percent,
        // } => try_update_mixnode_config(profit_margin_percent, info, deps),
        // ExecuteMsg::UpdateMixnetAddress { address } => {
        //     try_update_mixnet_address(address, info, deps)
        // }
        // ExecuteMsg::DelegateToMixnode {
        //     mix_identity,
        //     amount,
        // } => try_delegate_to_mixnode(mix_identity, amount, info, env, deps),
        // ExecuteMsg::UndelegateFromMixnode { mix_identity } => {
        //     try_undelegate_from_mixnode(mix_identity, info, deps)
        // }
        // ExecuteMsg::CreateAccount {
        //     owner_address,
        //     staking_address,
        //     vesting_spec,
        // } => try_create_periodic_vesting_account(
        //     &owner_address,
        //     staking_address,
        //     vesting_spec,
        //     info,
        //     env,
        //     deps,
        // ),
        // ExecuteMsg::WithdrawVestedCoins { amount } => {
        //     try_withdraw_vested_coins(amount, env, info, deps)
        // }
        //
        // ExecuteMsg::BondMixnode {
        //     mix_node,
        //     owner_signature,
        //     amount,
        // } => try_bond_mixnode(mix_node, owner_signature, amount, info, env, deps),
        // ExecuteMsg::UnbondMixnode {} => try_unbond_mixnode(info, deps),
        // ExecuteMsg::TrackUnbondMixnode { owner, amount } => {
        //     try_track_unbond_mixnode(&owner, amount, info, deps)
        // }
        // ExecuteMsg::BondGateway {
        //     gateway,
        //     owner_signature,
        //     amount,
        // } => try_bond_gateway(gateway, owner_signature, amount, info, env, deps),
        // ExecuteMsg::UnbondGateway {} => try_unbond_gateway(info, deps),
        // ExecuteMsg::TrackUnbondGateway { owner, amount } => {
        //     try_track_unbond_gateway(&owner, amount, info, deps)
        // }
        // ExecuteMsg::TransferOwnership { to_address } => {
        //     try_transfer_ownership(to_address, info, deps)
        // }
        // ExecuteMsg::UpdateStakingAddress { to_address } => {
        //     try_update_staking_address(to_address, info, deps)
        // }
    }
}

pub fn try_update_locked_pledge_cap(
    amount: Uint128,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    update_locked_pledge_cap(amount, deps.storage)?;
    Ok(Response::default())
}

pub fn try_update_mixnode_config(
    profit_margin_percent: u8,
    info: MessageInfo,
    deps: DepsMut,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_update_mixnode_config(profit_margin_percent, deps.storage)
}

// Only contract admin, set at init
pub fn try_update_mixnet_address(
    address: String,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    MIXNET_CONTRACT_ADDRESS.save(deps.storage, &address)?;
    Ok(Response::default())
}

// Only contract owner of vesting account
pub fn try_withdraw_vested_coins(
    amount: Coin,
    env: Env,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    if amount.denom != mix_denom {
        return Err(ContractError::WrongDenom(amount.denom, mix_denom));
    }

    let address = info.sender.clone();
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address != account.owner_address() {
        return Err(ContractError::NotOwner(account.owner_address().to_string()));
    }
    let spendable_coins = account.spendable_coins(None, &env, deps.storage)?;
    if amount.amount <= spendable_coins.amount {
        let new_balance = account.withdraw(&amount, deps.storage)?;

        let send_tokens = BankMsg::Send {
            to_address: account.owner_address().as_str().to_string(),
            amount: vec![amount.clone()],
        };

        Ok(Response::new()
            .add_message(send_tokens)
            .add_event(new_vested_coins_withdraw_event(
                &address,
                &amount,
                &coin(new_balance, &amount.denom),
            )))
    } else {
        Err(ContractError::InsufficientSpendable(
            account.owner_address().as_str().to_string(),
            spendable_coins.amount.u128(),
        ))
    }
}

fn try_transfer_ownership(
    to_address: String,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let address = info.sender.clone();
    let to_address = deps.api.addr_validate(&to_address)?;
    let mut account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address == account.owner_address() {
        account.transfer_ownership(&to_address, deps.storage)?;
        Ok(Response::new().add_event(new_ownership_transfer_event(&address, &to_address)))
    } else {
        Err(ContractError::NotOwner(account.owner_address().to_string()))
    }
}

fn try_update_staking_address(
    to_address: Option<String>,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let address = info.sender.clone();
    let to_address = to_address.and_then(|x| deps.api.addr_validate(&x).ok());
    let mut account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    if address == account.owner_address() {
        let old = account.staking_address().cloned();
        account.update_staking_address(to_address.clone(), deps.storage)?;
        Ok(Response::new().add_event(new_staking_address_update_event(&old, &to_address)))
    } else {
        Err(ContractError::NotOwner(account.owner_address().to_string()))
    }
}

// Owner or staking
pub fn try_bond_gateway(
    gateway: Gateway,
    owner_signature: String,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let pledge = validate_funds(&[amount], mix_denom)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_bond_gateway(gateway, owner_signature, pledge, &env, deps.storage)
}

pub fn try_unbond_gateway(info: MessageInfo, deps: DepsMut<'_>) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_gateway(deps.storage)
}

pub fn try_track_unbond_gateway(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_gateway(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_gateway_unbond_event()))
}

pub fn try_compound_operator_reward(
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_compound_operator_reward(deps.storage)
}

pub fn try_bond_mixnode(
    mix_node: MixNode,
    owner_signature: String,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let pledge = validate_funds(&[amount], mix_denom)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_bond_mixnode(mix_node, owner_signature, pledge, &env, deps.storage)
}

pub fn try_unbond_mixnode(info: MessageInfo, deps: DepsMut<'_>) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_unbond_mixnode(deps.storage)
}

pub fn try_track_unbond_mixnode(
    owner: &str,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(owner, deps.storage, deps.api)?;
    account.try_track_unbond_mixnode(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_mixnode_unbond_event()))
}

fn try_track_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    amount: Coin,
    address: &str,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.track_reward(amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_reward_event()))
}

fn try_track_undelegation(
    address: &str,
    mix_identity: IdentityKey,
    amount: Coin,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != MIXNET_CONTRACT_ADDRESS.load(deps.storage)? {
        return Err(ContractError::NotMixnetContract(info.sender));
    }
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.track_undelegation(mix_identity, amount, deps.storage)?;
    Ok(Response::new().add_event(new_track_undelegation_event()))
}

fn try_delegate_to_mixnode(
    mix_identity: IdentityKey,
    amount: Coin,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let mix_denom = MIX_DENOM.load(deps.storage)?;
    let amount = validate_funds(&[amount], mix_denom)?;
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_delegate_to_mixnode(mix_identity, amount, &env, deps.storage)
}

fn try_compound_delegator_reward(
    mix_identity: IdentityKey,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_compound_delegator_reward(mix_identity, deps.storage)
}

fn try_claim_operator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_claim_operator_reward(deps.storage)
}

fn try_claim_delegator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_identity: String,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_claim_delegator_reward(mix_identity, deps.storage)
}

fn try_undelegate_from_mixnode(
    mix_identity: IdentityKey,
    info: MessageInfo,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    let account = account_from_address(info.sender.as_str(), deps.storage, deps.api)?;
    account.try_undelegate_from_mixnode(mix_identity, deps.storage)
}

fn try_create_periodic_vesting_account(
    owner_address: &str,
    staking_address: Option<String>,
    vesting_spec: Option<VestingSpecification>,
    info: MessageInfo,
    env: Env,
    deps: DepsMut<'_>,
) -> Result<Response, ContractError> {
    if info.sender != ADMIN.load(deps.storage)? {
        return Err(ContractError::NotAdmin(info.sender.as_str().to_string()));
    }
    let mix_denom = MIX_DENOM.load(deps.storage)?;

    let account_exists = account_from_address(owner_address, deps.storage, deps.api).is_ok();
    if account_exists {
        return Err(ContractError::AccountAlreadyExists(
            owner_address.to_string(),
        ));
    }

    let vesting_spec = vesting_spec.unwrap_or_default();

    let coin = validate_funds(&info.funds, mix_denom)?;

    let owner_address = deps.api.addr_validate(owner_address)?;
    let staking_address = if let Some(staking_address) = staking_address {
        Some(deps.api.addr_validate(&staking_address)?)
    } else {
        None
    };
    let start_time = vesting_spec
        .start_time()
        .unwrap_or_else(|| env.block.time.seconds());

    let periods = populate_vesting_periods(start_time, vesting_spec);

    let start_time = Timestamp::from_seconds(start_time);

    let response = Response::new();

    Account::new(
        owner_address.clone(),
        staking_address.clone(),
        coin.clone(),
        start_time,
        periods,
        deps.storage,
    )?;

    Ok(response.add_event(new_periodic_vesting_account_event(
        &owner_address,
        &coin,
        &staking_address,
        start_time,
    )))
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetLockedPledgeCap {} => to_binary(&get_locked_pledge_cap(deps)),
        QueryMsg::LockedCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_locked_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::SpendableCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_spendable_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetVestedCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_vested_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetVestingCoins {
            vesting_account_address,
            block_time,
        } => to_binary(&try_get_vesting_coins(
            &vesting_account_address,
            block_time,
            env,
            deps,
        )?),
        QueryMsg::GetStartTime {
            vesting_account_address,
        } => to_binary(&try_get_start_time(&vesting_account_address, deps)?),
        QueryMsg::GetEndTime {
            vesting_account_address,
        } => to_binary(&try_get_end_time(&vesting_account_address, deps)?),
        QueryMsg::GetOriginalVesting {
            vesting_account_address,
        } => to_binary(&try_get_original_vesting(&vesting_account_address, deps)?),
        QueryMsg::GetDelegatedFree {
            block_time,
            vesting_account_address,
        } => to_binary(&try_get_delegated_free(
            block_time,
            &vesting_account_address,
            env,
            deps,
        )?),
        QueryMsg::GetDelegatedVesting {
            block_time,
            vesting_account_address,
        } => to_binary(&try_get_delegated_vesting(
            block_time,
            &vesting_account_address,
            env,
            deps,
        )?),
        QueryMsg::GetAccount { address } => to_binary(&try_get_account(&address, deps)?),
        QueryMsg::GetMixnode { address } => to_binary(&try_get_mixnode(&address, deps)?),
        QueryMsg::GetGateway { address } => to_binary(&try_get_gateway(&address, deps)?),
        QueryMsg::GetCurrentVestingPeriod { address } => {
            to_binary(&try_get_current_vesting_period(&address, deps, env)?)
        }
        QueryMsg::GetDelegationTimes {
            address,
            mix_identity,
        } => to_binary(&try_get_delegation_times(deps, &address, mix_identity)?),
        QueryMsg::GetAllDelegations { start_after, limit } => {
            to_binary(&try_get_all_delegations(deps, start_after, limit)?)
        }
    };

    Ok(query_res?)
}

pub fn get_locked_pledge_cap(deps: Deps<'_>) -> Uint128 {
    locked_pledge_cap(deps.storage)
}

pub fn try_get_current_vesting_period(
    address: &str,
    deps: Deps<'_>,
    env: Env,
) -> Result<Period, ContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    Ok(account.get_current_vesting_period(env.block.time))
}

pub fn try_get_mixnode(address: &str, deps: Deps<'_>) -> Result<Option<PledgeData>, ContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.load_mixnode_pledge(deps.storage)
}

pub fn try_get_gateway(address: &str, deps: Deps<'_>) -> Result<Option<PledgeData>, ContractError> {
    let account = account_from_address(address, deps.storage, deps.api)?;
    account.load_gateway_pledge(deps.storage)
}

pub fn try_get_account(address: &str, deps: Deps<'_>) -> Result<Account, ContractError> {
    account_from_address(address, deps.storage, deps.api)
}

pub fn try_get_locked_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.locked_coins(block_time, &env, deps.storage)
}

pub fn try_get_spendable_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.spendable_coins(block_time, &env, deps.storage)
}

pub fn try_get_vested_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vested_coins(block_time, &env, deps.storage)
}

pub fn try_get_vesting_coins(
    vesting_account_address: &str,
    block_time: Option<Timestamp>,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_vesting_coins(block_time, &env, deps.storage)
}

pub fn try_get_start_time(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Timestamp, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_start_time())
}

pub fn try_get_end_time(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<Timestamp, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_end_time())
}

pub fn try_get_original_vesting(
    vesting_account_address: &str,
    deps: Deps<'_>,
) -> Result<OriginalVestingResponse, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    Ok(account.get_original_vesting())
}

pub fn try_get_delegated_free(
    block_time: Option<Timestamp>,
    vesting_account_address: &str,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_delegated_free(block_time, &env, deps.storage)
}

pub fn try_get_delegated_vesting(
    block_time: Option<Timestamp>,
    vesting_account_address: &str,
    env: Env,
    deps: Deps<'_>,
) -> Result<Coin, ContractError> {
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;
    account.get_delegated_vesting(block_time, &env, deps.storage)
}

pub fn try_get_delegation_times(
    deps: Deps<'_>,
    vesting_account_address: &str,
    mix_identity: String,
) -> Result<DelegationTimesResponse, ContractError> {
    let owner = deps.api.addr_validate(vesting_account_address)?;
    let account = account_from_address(vesting_account_address, deps.storage, deps.api)?;

    let delegation_timestamps = DELEGATIONS
        .prefix((account.storage_key(), mix_identity.clone()))
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(DelegationTimesResponse {
        owner,
        account_id: account.storage_key(),
        mix_identity,
        delegation_timestamps,
    })
}

pub fn try_get_all_delegations(
    deps: Deps<'_>,
    start_after: Option<(u32, IdentityKey, BlockTimestampSecs)>,
    limit: Option<u32>,
) -> Result<AllDelegationsResponse, ContractError> {
    let limit = limit.unwrap_or(100).min(200) as usize;

    let start = start_after.map(Bound::exclusive);
    let delegations = DELEGATIONS
        .range(deps.storage, start, None, Order::Ascending)
        .map(|kv| {
            kv.map(
                |((account_id, mix_identity, block_timestamp), amount)| VestingDelegation {
                    account_id,
                    mix_identity,
                    block_timestamp,
                    amount,
                },
            )
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = if delegations.len() < limit {
        None
    } else {
        delegations
            .last()
            .map(|delegation| delegation.storage_key())
    };

    Ok(AllDelegationsResponse {
        delegations,
        start_next_after,
    })
}

fn validate_funds(funds: &[Coin], mix_denom: String) -> Result<Coin, ContractError> {
    if funds.is_empty() || funds[0].amount.is_zero() {
        return Err(ContractError::EmptyFunds);
    }

    if funds.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    if funds[0].denom != mix_denom {
        return Err(ContractError::WrongDenom(funds[0].denom.clone(), mix_denom));
    }

    Ok(funds[0].clone())
}
