use std::vec;

use cosmwasm_std::{
    Addr, Coin, Deps, DepsMut, Env, Reply, Response, StdError, StdResult, SubMsg, SubMsgResponse,
    Uint128,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

use crate::AssetTrait;

const REPLY_SAVE_CW20_ADDRESS: u64 = 14509;

/// Unwrap a `Reply` object to extract the response
/// TODO: Copied from larrys steakhouse. Move to protocol
pub(crate) fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

/// Save a cw20 address from an instantiation event to the storage as a struct of type `A`.
pub fn save_cw20_address<A: AssetTrait + From<Addr>>(
    deps: DepsMut,
    reply: Reply,
    item: Item<A>,
) -> StdResult<Response> {
    if reply.id == REPLY_SAVE_CW20_ADDRESS {
        let res = unwrap_reply(reply)?;
        let addr = parse_contract_addr_from_instantiate_event(deps.as_ref(), res)?;
        item.save(deps.storage, &addr.clone().into())?;
        Ok(Response::new()
            .add_attribute("action", "save_osmosis_denom")
            .add_attribute("addr", &addr))
    } else {
        Ok(Response::new())
    }
}

fn parse_contract_addr_from_instantiate_event(
    deps: Deps,
    response: SubMsgResponse,
) -> StdResult<Addr> {
    let event = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate")
        .ok_or_else(|| StdError::generic_err("cannot find `instantiate` event"))?;

    let contract_addr_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "_contract_address")
        .ok_or_else(|| StdError::generic_err("cannot find `_contract_address` attribute"))?
        .value;

    deps.api.addr_validate(contract_addr_str)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum TokenInitInfo {
    Osmosis {
        subdenom: String,
    },
    Cw20 {
        label: String,
        admin: Option<String>,
        code_id: u64,
        cw20_init_msg: Box<Cw20InstantiateMsg>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AAssetInstantiator {
    pub item_key: String,
    pub init_info: TokenInitInfo,
}

pub const TOKEN_ITEM_KEY: Item<String> = Item::new("token_item_key");

pub trait AssetInstantiator<A> {
    fn instantiate<B: Into<A>>(&self, deps: DepsMut, env: Env, msg: B) -> StdResult<SubMsg>;
}

/// Find the amount of a denom sent along a message, assert it is non-zero, and no other denom were
/// sent together
/// TODO: Took from steakcontracts. Move out to protocol utils and use here and in main steak contracts
pub(crate) fn parse_received_fund(funds: &[Coin], denom: &str) -> StdResult<Uint128> {
    if funds.len() != 1 {
        return Err(StdError::generic_err(format!(
            "must deposit exactly one coin; received {}",
            funds.len()
        )));
    }

    let fund = &funds[0];
    if fund.denom != denom {
        return Err(StdError::generic_err(format!(
            "expected {} deposit, received {}",
            denom, fund.denom
        )));
    }

    if fund.amount.is_zero() {
        return Err(StdError::generic_err("deposit amount must be non-zero"));
    }

    Ok(fund.amount)
}
