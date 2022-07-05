use std::convert::TryFrom;

use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdError, StdResult, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Asset, AssetInfo, Burn, Mint, Transferable};

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
