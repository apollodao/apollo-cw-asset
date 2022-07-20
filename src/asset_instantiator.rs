use crate::CwAssetError;
use cosmwasm_std::{
    Api, DepsMut, Reply, Response, StdError, StdResult, Storage, SubMsg, SubMsgResponse,
};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::vec;

/// Unwrap a `Reply` object to extract the response
pub(crate) fn unwrap_reply(reply: &Reply) -> StdResult<SubMsgResponse> {
    reply.clone().result.into_result().map_err(StdError::generic_err)
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
    fn instantiate_msg(&self, deps: DepsMut) -> StdResult<SubMsg>;

    fn save_asset(
        storage: &mut dyn Storage,
        api: &dyn Api,
        reply: &Reply,
        item: Item<A>,
    ) -> Result<Response, CwAssetError>;
}
