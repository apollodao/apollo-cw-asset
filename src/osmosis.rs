use std::convert::TryFrom;

use cosmwasm_std::{
    to_binary, Api, Coin, CosmosMsg, DepsMut, Env, Reply, Response, StdError, StdResult, Storage,
    SubMsg, SubMsgResponse, Uint128,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    unwrap_reply, Asset, AssetInfo, Burn, Instantiate, Mint, Transferable, TOKEN_ITEM_KEY,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OsmosisCoin(Coin);

impl From<OsmosisCoin> for Asset {
    fn from(asset: OsmosisCoin) -> Asset {
        Asset::from(asset.0)
    }
}

impl TryFrom<Asset> for OsmosisCoin {
    type Error = StdError;

    fn try_from(asset: Asset) -> StdResult<Self> {
        match asset.info {
            AssetInfo::Cw20(_) => {
                Err(StdError::generic_err("Cannot convert Cw20 asset to OsmosisDenom."))
            }
            AssetInfo::Native(denom) => {
                let parts: Vec<&str> = denom.split('/').collect();
                if parts.len() != 3 || parts[0] != "factory" {
                    return Err(StdError::generic_err("Invalid denom for OsmosisDenom."));
                }
                Ok(OsmosisCoin(Coin::new(asset.amount.into(), denom)))
            }
        }
    }
}

impl Transferable for OsmosisCoin {}

impl Mint for OsmosisCoin {
    fn mint_msg<A: Into<String>, B: Into<String>>(
        &self,
        sender: A,
        _recipient: B,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Stargate {
            type_url: "/osmosis.tokenfactory.v1beta1.MsgMint".to_string(),
            value: to_binary(&OsmosisMintMsg {
                amount: Coin {
                    denom: self.0.denom.clone(),
                    amount: self.0.amount,
                },
                sender: sender.into(),
            })?,
        })
    }
}

impl Burn for OsmosisCoin {
    fn burn_msg<A: Into<String>>(&self, sender: A) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Stargate {
            type_url: "/osmosis.tokenfactory.v1beta1.MsgBurn".to_string(),
            value: to_binary(&OsmosisBurnMsg {
                amount: Coin {
                    denom: self.0.denom.clone(),
                    amount: self.0.amount,
                },
                sender: sender.into(),
            })?,
        })
    }
}

// TODO: Fix stargate to use .proto files and remove these structs
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct OsmosisMintMsg {
    amount: Coin,
    sender: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct OsmosisBurnMsg {
    amount: Coin,
    sender: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OsmosisCreateDenomMsg {
    sender: String,
    subdenom: String,
}

pub type OsmosisDenomInstantiator = String;

impl Instantiate<AssetInfo> for OsmosisDenomInstantiator {
    fn instantiate_msg(&self, deps: DepsMut, env: Env) -> StdResult<SubMsg> {
        Ok(SubMsg::reply_always(
            CosmosMsg::Stargate {
                type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".to_string(),
                value: to_binary(&OsmosisCreateDenomMsg {
                    sender: env.contract.address.to_string(),
                    subdenom: self.clone(),
                })?,
            },
            REPLY_SAVE_OSMOSIS_DENOM,
        ))
    }

    fn save_asset(
        storage: &mut dyn Storage,
        api: &dyn Api,
        reply: &Reply,
        item: Item<AssetInfo>,
    ) -> StdResult<Response> {
        match reply.id {
            REPLY_SAVE_OSMOSIS_DENOM => {
                let res = unwrap_reply(reply)?;
                let osmosis_denom = parse_osmosis_denom_from_instantiate_event(res)
                    .map_err(|e| StdError::generic_err(format!("{}", e)))?;

                item.save(storage, &AssetInfo::Native(osmosis_denom.clone()))?;

                Ok(Response::new()
                    .add_attribute("action", "save_osmosis_denom")
                    .add_attribute("denom", &osmosis_denom))
            }
            _ => Err(StdError::generic_err("Unexpected reply id")),
        }
    }
}

const REPLY_SAVE_OSMOSIS_DENOM: u64 = 14508;

fn parse_osmosis_denom_from_instantiate_event(response: SubMsgResponse) -> StdResult<String> {
    let event = response
        .events
        .iter()
        .find(|event| event.ty == "create_denom")
        .ok_or_else(|| StdError::generic_err("cannot find `create_denom` event"))?;

    let denom = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "new_token_denom")
        .ok_or_else(|| StdError::generic_err("cannot find `new_token_denom` attribute"))?
        .value;

    Ok(denom.to_string())
}

// TODO:
// * Implement TryFrom<Asset> for OsmosisDenom
//     * Verify valid denom
// * Implement From<OsmosisDenom> for Asset
// * Break out minting and burning into separate trait and implement cw20token
// * Verify owner function on OsmosisDenom
// * More useful functions?
// * Implement queries as trait
