use crate::{Asset, AssetInfo};
use cosmwasm_std::{Addr, QuerierWrapper, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure describes the main controls configs of pair
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairInfo {
    /// asset infos of assets in pair, available in [`AssetInfo`]
    pub asset_infos: [AssetInfo; 2],
    /// pair contract address
    pub contract_addr: Addr,
    /// pair liquidity token (optional)
    pub liquidity_token: Option<Addr>,
    /// type of pair, available in [`PairType`]
    pub pair_type: PairType,
}

impl PairInfo {
    /// ## Description
    /// Returns balance for each asset in the pool.
    /// ## Params
    /// * **self** is the type of the caller object
    ///
    /// * **querier** is the object of type [`QuerierWrapper`]
    ///
    /// * **contract_addr** is the pool address of the pair.
    pub fn query_pools(
        &self,
        querier: &QuerierWrapper,
        contract_addr: Addr,
    ) -> StdResult<[Asset; 2]> {
        Ok([
            Asset {
                amount: self.asset_infos[0].query_balance(querier, contract_addr.clone())?,
                info: self.asset_infos[0].clone(),
            },
            Asset {
                amount: self.asset_infos[1].query_balance(querier, contract_addr)?,
                info: self.asset_infos[1].clone(),
            },
        ])
    }
}

/// ## Description
/// This structure describes Pair types
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PairType {
    /// XYK pair type
    Xyk {},
    /// Stable pair type
    Stable {},
    /// No pair type (used by Router to price pairs of assets without an LP)
    None {},
}

impl PairType {
    pub fn is_none(&self) -> bool {
        match self {
            PairType::None {} => true,
            _ => false,
        }
    }
}

/// ## Description
/// Calculates key of pair from the specified parameters in the `asset_infos` variable.
/// ## Params
/// `asset_infos` it is array with two items the type of [`AssetInfo`].
pub fn pair_key(asset_infos: &[AssetInfo; 2]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat()
}
