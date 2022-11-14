use std::fmt;
use std::fmt::Formatter;

use cosmwasm_std::{
    to_binary, Addr, Api, BalanceResponse, BankQuery, QuerierWrapper, QueryRequest, StdResult,
    Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfoBase<T> {
    Cw20(T),        // the contract address, String or cosmwasm_std::Addr
    Native(String), // the native token's denom
}

pub type AssetInfoUnchecked = AssetInfoBase<String>;
pub type AssetInfo = AssetInfoBase<Addr>;

impl From<AssetInfo> for AssetInfoUnchecked {
    fn from(asset_info: AssetInfo) -> Self {
        match &asset_info {
            AssetInfo::Cw20(contract_addr) => AssetInfoUnchecked::Cw20(contract_addr.into()),
            AssetInfo::Native(denom) => AssetInfoUnchecked::Native(denom.clone()),
        }
    }
}

impl AssetInfoUnchecked {
    /// Validate contract address (if any) and returns a new `AssetInfo` instance
    pub fn check(&self, api: &dyn Api) -> StdResult<AssetInfo> {
        Ok(match self {
            AssetInfoUnchecked::Cw20(contract_addr) => {
                AssetInfo::Cw20(api.addr_validate(contract_addr)?)
            }
            AssetInfoUnchecked::Native(denom) => AssetInfo::Native(denom.clone()),
        })
    }
}

impl fmt::Display for AssetInfoUnchecked {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AssetInfoUnchecked::Cw20(contract_addr) => write!(f, "{}", contract_addr),
            AssetInfoUnchecked::Native(denom) => write!(f, "{}", denom),
        }
    }
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetInfo::Cw20(contract_addr) => write!(f, "{}", contract_addr),
            AssetInfo::Native(denom) => write!(f, "{}", denom),
        }
    }
}

#[cfg(feature = "astroport")]
impl From<astroport_core::asset::AssetInfo> for AssetInfo {
    fn from(astro_asset: astroport_core::asset::AssetInfo) -> Self {
        match astro_asset {
            astroport_core::asset::AssetInfo::Token {
                contract_addr,
            } => Self::cw20(contract_addr),
            astroport_core::asset::AssetInfo::NativeToken {
                denom,
            } => Self::native(denom),
        }
    }
}

#[cfg(feature = "astroport")]
impl From<AssetInfo> for astroport_core::asset::AssetInfo {
    fn from(astro_asset: AssetInfo) -> Self {
        match astro_asset {
            AssetInfo::Cw20(contract_addr) => astroport_core::asset::AssetInfo::Token {
                contract_addr,
            },
            AssetInfo::Native(denom) => astroport_core::asset::AssetInfo::NativeToken {
                denom,
            },
        }
    }
}

impl AssetInfo {
    /// Create a new `AssetInfoBase` instance representing a CW20 token of given contract address
    pub fn cw20<A: Into<Addr>>(contract_addr: A) -> Self {
        AssetInfo::Cw20(contract_addr.into())
    }

    /// Create a new `AssetInfoBase` instance representing a native token of given denom
    pub fn native<A: Into<String>>(denom: A) -> Self {
        AssetInfo::Native(denom.into())
    }

    pub fn from_str(api: &dyn Api, s: &str) -> Self {
        match api.addr_validate(s) {
            Ok(contract_addr) => AssetInfo::cw20(contract_addr),
            Err(_) => AssetInfo::native(s.to_string()),
        }
    }

    /// Query an address' balance of the asset
    pub fn query_balance<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        address: T,
    ) -> StdResult<Uint128> {
        match self {
            AssetInfo::Cw20(contract_addr) => {
                let response: Cw20BalanceResponse =
                    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: contract_addr.into(),
                        msg: to_binary(&Cw20QueryMsg::Balance {
                            address: address.into(),
                        })?,
                    }))?;
                Ok(response.balance)
            }
            AssetInfo::Native(denom) => {
                let response: BalanceResponse =
                    querier.query(&QueryRequest::Bank(BankQuery::Balance {
                        address: address.into(),
                        denom: denom.clone(),
                    }))?;
                Ok(response.amount.amount)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::MockApi;

    #[test]
    fn creating_instances() {
        let info = AssetInfo::cw20(Addr::unchecked("mock_token"));
        assert_eq!(info, AssetInfo::Cw20(Addr::unchecked("mock_token")));

        let info = AssetInfo::native("uusd");
        assert_eq!(info, AssetInfo::Native(String::from("uusd")));
    }

    #[test]
    fn comparing() {
        let uluna = AssetInfo::native("uluna");
        let uusd = AssetInfo::native("uusd");
        let astro = AssetInfo::cw20(Addr::unchecked("astro_token"));
        let mars = AssetInfo::cw20(Addr::unchecked("mars_token"));

        assert_eq!(uluna == uusd, false);
        assert_eq!(uluna == astro, false);
        assert_eq!(astro == mars, false);
        assert_eq!(uluna == uluna.clone(), true);
        assert_eq!(astro == astro.clone(), true);
    }

    #[test]
    fn displaying() {
        let info = AssetInfo::native("uusd");
        assert_eq!(info.to_string(), String::from("uusd"));

        let info = AssetInfo::cw20(Addr::unchecked("mock_token"));
        assert_eq!(info.to_string(), String::from("mock_token"));
    }

    #[test]
    fn checking() {
        let api = MockApi::default();

        let checked = AssetInfo::cw20(Addr::unchecked("mock_token"));
        let unchecked: AssetInfoUnchecked = checked.clone().into();

        assert_eq!(unchecked.check(&api).unwrap(), checked);
    }
}
