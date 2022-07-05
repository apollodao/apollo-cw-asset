use std::vec;

use cosmwasm_std::{
    Addr, Deps, DepsMut, Env, Reply, Response, StdError, StdResult, SubMsg, SubMsgResponse,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

/// Unwrap a `Reply` object to extract the response
/// TODO: Copied from larrys steakhouse. Move to protocol
pub(crate) fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
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

pub const TOKEN_ITEM_KEY: Item<String> = Item::new("token_item_key");

pub trait Instantiate<A: Serialize + DeserializeOwned>: Sized {
    fn instantiate_msg(&self, deps: DepsMut, env: Env) -> StdResult<SubMsg>;

    fn save_asset(deps: DepsMut, reply: Reply, item: Item<A>) -> StdResult<Response>;
}
