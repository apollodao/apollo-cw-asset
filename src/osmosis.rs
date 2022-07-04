use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, DepsMut, Env, Reply, Response, StdError, StdResult, SubMsg,
    SubMsgResponse,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{unwrap_reply, AssetInstantiator, AssetTrait, TOKEN_ITEM_KEY};

pub type OsmosisDenom = Coin;

impl AssetTrait for OsmosisDenom {
    fn mint_msg<A: Into<String>, B: Into<String>>(
        &self,
        sender: A,
        recipient: B,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Stargate {
            type_url: "/osmosis.tokenfactory.v1beta1.MsgMint".to_string(),
            value: to_binary(&OsmosisMintMsg {
                amount: Coin {
                    denom: self.denom.clone(),
                    amount: self.amount,
                },
                sender: sender.into(),
            })?,
        })
    }

    fn burn_msg<A: Into<String>>(&self, sender: A) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Stargate {
            type_url: "/osmosis.tokenfactory.v1beta1.Msg/Burn".to_string(),
            value: to_binary(&OsmosisBurnMsg {
                amount: Coin {
                    denom: self.denom.clone(),
                    amount: self.amount,
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

pub struct OsmosisDenomInitMsg {
    pub item_key: String,
    pub subdenom: String,
}

impl AssetInstantiator<OsmosisDenomInitMsg> for OsmosisDenom {
    fn instantiate<A: Into<OsmosisDenomInitMsg>>(
        &self,
        deps: DepsMut,
        env: Env,
        msg: A,
    ) -> StdResult<SubMsg> {
        let osmosis_denom_init_msg = msg.into();

        TOKEN_ITEM_KEY.save(deps.storage, &osmosis_denom_init_msg.item_key)?;

        Ok(SubMsg::reply_always(
            CosmosMsg::Stargate {
                type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".to_string(),
                value: to_binary(&OsmosisCreateDenomMsg {
                    sender: env.contract.address.to_string(),
                    subdenom: osmosis_denom_init_msg.subdenom,
                })?,
            },
            REPLY_SAVE_OSMOSIS_DENOM,
        ))
    }
}

const REPLY_SAVE_OSMOSIS_DENOM: u64 = 14508;

/// Save a osmosis denom from an instantiation event to the storage as a struct of type `A`.
pub fn save_osmosis_denom<A: AssetTrait + From<String>>(
    deps: DepsMut,
    reply: Reply,
    item: Item<A>,
) -> StdResult<Response> {
    if reply.id == REPLY_SAVE_OSMOSIS_DENOM {
        let res = unwrap_reply(reply)?;
        let osmosis_denom = parse_osmosis_denom_from_instantiate_event(res)
            .map_err(|e| StdError::generic_err(format!("{}", e)))?;

        item.save(deps.storage, &osmosis_denom.clone().into())?;

        Ok(Response::new()
            .add_attribute("action", "save_osmosis_denom")
            .add_attribute("denom", &osmosis_denom))
    } else {
        Ok(Response::new())
    }
}

fn parse_osmosis_denom_from_instantiate_event(response: SubMsgResponse) -> StdResult<String> {
    let event = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate")
        .ok_or_else(|| StdError::generic_err("cannot find `instantiate` event"))?;

    let denom = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "new_token_denom")
        .ok_or_else(|| StdError::generic_err("cannot find `_contract_address` attribute"))?
        .value;

    Ok(denom.to_string())
}
