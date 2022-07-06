use std::convert::TryFrom;

use cosmwasm_std::{
    to_binary, Addr, Api, CosmosMsg, Deps, DepsMut, Env, Reply, Response, StdError, StdResult,
    Storage, SubMsg, SubMsgResponse, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

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

fn parse_contract_addr_from_instantiate_event(
    api: &dyn Api,
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

    api.addr_validate(contract_addr_str)
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

pub struct Cw20AssetInstantiator {
    pub label: String,
    pub admin: Option<String>,
    pub code_id: u64,
    pub cw20_init_msg: Cw20InstantiateMsg,
}

impl Instantiate<AssetInfo> for Cw20AssetInstantiator {
    fn instantiate_msg(&self, _deps: DepsMut, _env: Env) -> StdResult<SubMsg> {
        Ok(SubMsg::reply_always(
            WasmMsg::Instantiate {
                admin: self.admin.clone(),
                code_id: self.code_id,
                msg: to_binary(&self.cw20_init_msg)?,
                funds: vec![],
                label: self.label.clone(),
            },
            REPLY_SAVE_CW20_ADDRESS,
        ))
    }

    fn save_asset(
        storage: &mut dyn Storage,
        api: &dyn Api,
        reply: Reply,
        item: Item<AssetInfo>,
    ) -> StdResult<Response> {
        if reply.id == REPLY_SAVE_CW20_ADDRESS {
            let res = unwrap_reply(reply)?;
            let asset = parse_contract_addr_from_instantiate_event(api, res)?;

            item.save(storage, &asset.clone().into())?;
            Ok(Response::new()
                .add_attribute("action", "save_osmosis_denom")
                .add_attribute("addr", &asset))
        } else {
            Ok(Response::new())
        }
    }
}
