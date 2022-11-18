//! Contains versions of Asset and AssetInfo that Astroport uses and conversion
//! functions to the normal versions.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, BalanceResponse, BankQuery, QuerierWrapper, QueryRequest, StdError, StdResult,
    Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};

use crate::{Asset, AssetInfo, AssetList};

#[cw_serde]
pub enum AstroAssetInfo {
    Token {
        contract_addr: Addr,
    },
    NativeToken {
        denom: String,
    },
}

#[cw_serde]
pub struct AstroAsset {
    pub info: AstroAssetInfo,
    pub amount: Uint128,
}

impl AstroAssetInfo {
    pub fn query_pool(&self, querier: &QuerierWrapper, pool_addr: Addr) -> StdResult<Uint128> {
        match self {
            AstroAssetInfo::Token {
                contract_addr,
                ..
            } => {
                let res: Cw20BalanceResponse = querier
                    .query(&QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: String::from(contract_addr),
                        msg: to_binary(&Cw20QueryMsg::Balance {
                            address: String::from(pool_addr),
                        })?,
                    }))
                    .unwrap_or_else(|_| Cw20BalanceResponse {
                        balance: Uint128::zero(),
                    });

                Ok(res.balance)
            }
            AstroAssetInfo::NativeToken {
                denom,
                ..
            } => {
                let balance: BalanceResponse =
                    querier.query(&QueryRequest::Bank(BankQuery::Balance {
                        address: String::from(pool_addr),
                        denom: denom.to_string(),
                    }))?;
                Ok(balance.amount.amount)
            }
        }
    }
}

impl From<AstroAssetInfo> for AssetInfo {
    fn from(astro_asset: AstroAssetInfo) -> Self {
        match astro_asset {
            AstroAssetInfo::Token {
                contract_addr,
            } => Self::cw20(contract_addr),
            AstroAssetInfo::NativeToken {
                denom,
            } => Self::native(denom),
        }
    }
}

impl From<AssetInfo> for AstroAssetInfo {
    fn from(astro_asset: AssetInfo) -> Self {
        match astro_asset {
            AssetInfo::Cw20(contract_addr) => AstroAssetInfo::Token {
                contract_addr,
            },
            AssetInfo::Native(denom) => AstroAssetInfo::NativeToken {
                denom,
            },
        }
    }
}

impl From<&AstroAsset> for Asset {
    fn from(astro_asset: &AstroAsset) -> Self {
        Self {
            info: astro_asset.info.to_owned().into(),
            amount: astro_asset.amount,
        }
    }
}

impl From<Asset> for AstroAsset {
    fn from(astro_asset: Asset) -> Self {
        Self {
            info: astro_asset.info.into(),
            amount: astro_asset.amount,
        }
    }
}

impl From<Vec<AstroAsset>> for AssetList {
    fn from(astro_assets: Vec<AstroAsset>) -> Self {
        Self(astro_assets.iter().map(|asset| asset.into()).collect())
    }
}

impl std::convert::TryFrom<AssetList> for [AstroAsset; 2] {
    type Error = StdError;

    fn try_from(value: AssetList) -> Result<[AstroAsset; 2], Self::Error> {
        if value.len() != 2 {
            return Err(StdError::generic_err(format!(
                "AssetList must contain exactly 2 assets, but it contains {}",
                value.len()
            )));
        }
        let astro_assets = value.to_vec();
        Ok([astro_assets[0].to_owned().into(), astro_assets[1].to_owned().into()])
    }
}
