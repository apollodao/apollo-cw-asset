use crate::{
    unwrap_reply, Asset, AssetInfo, Burn, CwAssetError, Instantiate, IsNative, Mint, Transferable,
    TOKEN_ITEM_KEY,
};
use apollo_proto_rust::cosmos::base::v1beta1::Coin as CoinMsg;
use apollo_proto_rust::osmosis::tokenfactory::v1beta1::{MsgBurn, MsgCreateDenom, MsgMint};
use apollo_proto_rust::utils::encode;
use apollo_proto_rust::OsmosisTypeURLs;
use cosmwasm_std::{
    to_binary, Api, Binary, Coin, CosmosMsg, DepsMut, Env, Reply, Response, StdError, StdResult,
    Storage, SubMsg, SubMsgResponse, Uint128,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OsmosisCoin(pub Coin);

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

impl TryFrom<&Asset> for OsmosisCoin {
    type Error = StdError;

    fn try_from(asset: &Asset) -> StdResult<Self> {
        Self::try_from(asset.clone())
    }
}

impl IsNative for OsmosisCoin {
    fn is_native(&self) -> bool {
        true
    }
}

impl Transferable for OsmosisCoin {}

impl Mint for OsmosisCoin {
    fn mint_msgs<A: Into<String>, B: Into<String>>(
        &self,
        sender: A,
        recipient: B,
    ) -> StdResult<Vec<CosmosMsg>> {
        Ok(vec![
            CosmosMsg::Stargate {
                type_url: OsmosisTypeURLs::Mint.to_string(),
                value: encode(MsgMint {
                    amount: Some(CoinMsg {
                        denom: self.0.denom.to_string(),
                        amount: self.0.amount.to_string(),
                    }),
                    sender: sender.into(),
                }),
            },
            self.transfer_msg(recipient)?,
        ])
    }
}

impl Burn for OsmosisCoin {
    fn burn_msg<A: Into<String>>(&self, sender: A) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Stargate {
            type_url: OsmosisTypeURLs::Burn.to_string(),
            value: encode(MsgBurn {
                amount: Some(CoinMsg {
                    denom: self.0.denom.to_string(),
                    amount: self.0.amount.to_string(),
                }),
                sender: sender.into(),
            }),
        })
    }
}

pub type OsmosisDenomInstantiator = String;

impl Instantiate<AssetInfo> for OsmosisDenomInstantiator {
    fn instantiate_msg(&self, deps: DepsMut, env: Env) -> StdResult<SubMsg> {
        Ok(SubMsg::reply_always(
            CosmosMsg::Stargate {
                type_url: OsmosisTypeURLs::CreateDenom.to_string(),
                value: encode(MsgCreateDenom {
                    sender: env.contract.address.to_string(),
                    subdenom: self.clone(),
                }),
            },
            REPLY_SAVE_OSMOSIS_DENOM,
        ))
    }

    fn save_asset(
        storage: &mut dyn Storage,
        api: &dyn Api,
        reply: &Reply,
        item: Item<AssetInfo>,
    ) -> Result<Response, CwAssetError> {
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
            _ => Err(CwAssetError::InvalidReplyId {}),
        }
    }
}

pub const REPLY_SAVE_OSMOSIS_DENOM: u64 = 14508;

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
