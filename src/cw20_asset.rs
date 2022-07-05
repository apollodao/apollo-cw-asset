use std::convert::TryFrom;

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Deps, DepsMut, Env, Reply, Response, StdError, StdResult, SubMsg,
    SubMsgResponse, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

use crate::{unwrap_reply, Asset, AssetInfo, Burn, Instantiate, Mint, Transferable};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw20Asset {
    pub address: Addr,
    pub amount: Uint128,
}

impl From<Cw20Asset> for Asset {
    fn from(cw20_asset: Cw20Asset) -> Self {
        Asset {
            amount: cw20_asset.amount,
            info: AssetInfo::Cw20(cw20_asset.address),
        }
    }
}

impl TryFrom<Asset> for Cw20Asset {
    type Error = StdError;

    fn try_from(asset: Asset) -> StdResult<Self> {
        match asset.info {
            AssetInfo::Cw20(address) => Ok(Cw20Asset {
                address,
                amount: asset.amount,
            }),
            AssetInfo::Native(_) => {
                Err(StdError::generic_err("Cannot convert native asset to Cw20."))
            }
        }
    }
}

// ------ Implement Instantiate for Cw20Asset ------

const REPLY_SAVE_CW20_ADDRESS: u64 = 14509;

/// Save a cw20 address from an instantiation event to the storage as a struct of type `A`.
pub fn save_cw20_address<A: Transferable + From<Addr>>(
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

pub struct Cw20AssetInitMsg {
    pub label: String,
    pub admin: Option<String>,
    pub code_id: u64,
    pub cw20_init_msg: Cw20InstantiateMsg,
}

impl Instantiate<Cw20AssetInitMsg> for Cw20Asset {
    fn instantiate_msg<B: Into<Cw20AssetInitMsg>>(
        &self,
        _deps: DepsMut,
        _env: Env,
        msg: B,
    ) -> StdResult<SubMsg> {
        let msg: Cw20AssetInitMsg = msg.into();
        Ok(SubMsg::reply_always(
            WasmMsg::Instantiate {
                admin: msg.admin,
                code_id: msg.code_id,
                msg: to_binary(&msg.cw20_init_msg)?,
                funds: vec![],
                label: msg.label,
            },
            REPLY_SAVE_CW20_ADDRESS,
        ))
    }
}

impl Transferable for Cw20Asset {}

impl Mint for Cw20Asset {
    fn mint_msg<A: Into<String>, B: Into<String>>(
        &self,
        _sender: A,
        recipient: B,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: recipient.into(),
                amount: self.amount,
            })?,
            funds: vec![],
        }))
    }
}

impl Burn for Cw20Asset {
    fn burn_msg<A: Into<String>>(&self, sender: A) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: self.amount,
            })?,
            funds: vec![],
        }))
    }
}
