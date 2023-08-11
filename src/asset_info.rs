use std::convert::TryFrom;
use std::fmt;
use std::fmt::Formatter;

use cosmwasm_std::{
    to_binary, Addr, Api, BalanceResponse, BankQuery, QuerierWrapper, QueryRequest, StdError,
    StdResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, Denom};

use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, JsonSchema)]
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

impl From<&AssetInfo> for AssetInfoUnchecked {
    fn from(asset_info: &AssetInfo) -> Self {
        asset_info.clone().into()
    }
}

impl From<Denom> for AssetInfo {
    fn from(denom: Denom) -> Self {
        match denom {
            Denom::Cw20(contract_addr) => AssetInfo::Cw20(contract_addr),
            Denom::Native(denom) => AssetInfo::Native(denom),
        }
    }
}

impl From<AssetInfo> for Denom {
    fn from(asset_info: AssetInfo) -> Self {
        match asset_info {
            AssetInfo::Cw20(contract_addr) => Denom::Cw20(contract_addr),
            AssetInfo::Native(denom) => Denom::Native(denom),
        }
    }
}

#[cfg(feature = "astroport")]
impl From<astroport::asset::AssetInfo> for AssetInfo {
    fn from(value: astroport::asset::AssetInfo) -> Self {
        match value {
            astroport::asset::AssetInfo::Token { contract_addr } => AssetInfo::Cw20(contract_addr),
            astroport::asset::AssetInfo::NativeToken { denom } => AssetInfo::Native(denom),
        }
    }
}

#[cfg(feature = "astroport")]
impl From<AssetInfo> for astroport::asset::AssetInfo {
    fn from(value: AssetInfo) -> Self {
        match value {
            AssetInfoBase::Cw20(addr) => astroport::asset::AssetInfo::Token {
                contract_addr: addr,
            },
            AssetInfoBase::Native(denom) => astroport::asset::AssetInfo::NativeToken { denom },
        }
    }
}

impl AssetInfoUnchecked {
    /// Validate contract address (if any) and returns a new `AssetInfo`
    /// instance
    pub fn check(&self, api: &dyn Api) -> StdResult<AssetInfo> {
        Ok(match self {
            AssetInfoUnchecked::Cw20(contract_addr) => {
                AssetInfo::Cw20(api.addr_validate(contract_addr)?)
            }
            AssetInfoUnchecked::Native(denom) => AssetInfo::Native(denom.clone()),
        })
    }

    pub fn native<A: Into<String>>(denom: A) -> Self {
        AssetInfoUnchecked::Native(denom.into())
    }

    pub fn cw20<A: Into<String>>(contract_addr: A) -> Self {
        AssetInfoUnchecked::Cw20(contract_addr.into())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AssetInfoKey {
    bytes: Vec<u8>,
}

impl AssetInfoKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<AssetInfo> for AssetInfoKey {
    fn from(asset_info: AssetInfo) -> Self {
        let mut bytes = Vec::new();
        match asset_info {
            AssetInfo::Cw20(contract_addr) => {
                bytes.push(u8::MIN);
                bytes.append(&mut contract_addr.as_bytes().to_vec());
            }
            AssetInfo::Native(denom) => {
                bytes.push(u8::MAX);
                bytes.append(&mut denom.as_bytes().to_vec());
            }
        }
        AssetInfoKey { bytes }
    }
}

impl From<&AssetInfo> for AssetInfoKey {
    fn from(asset_info: &AssetInfo) -> Self {
        asset_info.clone().into()
    }
}

impl From<AssetInfoKey> for AssetInfo {
    fn from(asset_info_key: AssetInfoKey) -> Self {
        let bytes = asset_info_key.bytes;
        let first_byte = bytes[0];
        let rest = String::from_utf8(bytes[1..].to_vec()).unwrap();
        match first_byte {
            u8::MIN => AssetInfo::Cw20(Addr::unchecked(rest)),
            u8::MAX => AssetInfo::Native(rest),
            _ => panic!("Invalid AssetInfoKey"),
        }
    }
}

impl TryFrom<AssetInfo> for Addr {
    type Error = StdError;

    fn try_from(asset_info: AssetInfo) -> StdResult<Self> {
        match asset_info {
            AssetInfo::Cw20(contract_addr) => Ok(contract_addr),
            AssetInfo::Native(_) => Err(StdError::generic_err("Not a CW20 token")),
        }
    }
}

impl From<Addr> for AssetInfo {
    fn from(contract_addr: Addr) -> Self {
        AssetInfo::Cw20(contract_addr)
    }
}

impl PartialEq<AssetInfo> for AssetInfoKey {
    fn eq(&self, other: &AssetInfo) -> bool {
        self == &AssetInfoKey::from(other)
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

impl<'a> PrimaryKey<'a> for AssetInfoKey {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<cw_storage_plus::Key> {
        vec![Key::Ref(&self.bytes)]
    }
}

impl KeyDeserialize for AssetInfoKey {
    type Output = Self;

    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(Self { bytes: value })
    }
}

impl<'a> Prefixer<'a> for AssetInfoKey {
    fn prefix(&self) -> Vec<cw_storage_plus::Key> {
        vec![Key::Ref(&self.bytes)]
    }
}

impl AssetInfo {
    /// Create a new `AssetInfoBase` instance representing a CW20 token of given
    /// contract address
    pub fn cw20<A: Into<Addr>>(contract_addr: A) -> Self {
        AssetInfo::Cw20(contract_addr.into())
    }

    /// Create a new `AssetInfoBase` instance representing a native token of
    /// given denom
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

    pub fn is_native(&self) -> bool {
        matches!(self, AssetInfo::Native(_))
    }

    /// Create a new asset from the `AssetInfo` with the given amount
    pub fn to_asset(&self, amount: impl Into<Uint128>) -> Asset {
        Asset {
            info: self.clone(),
            amount: amount.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::convert::TryInto;

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

        assert!(uluna != uusd);
        assert!(uluna != astro);
        assert!(astro != mars);
        assert!(uluna == uluna.clone());
        assert!(astro == astro.clone());
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

    #[test]
    fn test_from_addr() {
        let addr = Addr::unchecked("mock_token");
        let info = AssetInfo::from(addr.clone());
        assert_eq!(info, AssetInfo::Cw20(addr));
    }

    #[test]
    fn test_try_from_asset_info_for_addr() {
        let addr = Addr::unchecked("mock_token");
        let info = AssetInfo::Cw20(addr.clone());
        let addr2: Addr = info.try_into().unwrap();
        assert_eq!(addr, addr2);
    }

    #[test]
    fn native_asset_info() {
        let info = AssetInfo::native("uusd");
        assert_eq!(AssetInfo::Native("uusd".to_string()), info);
    }

    #[test]
    fn cw20_asset_info() {
        let info = AssetInfo::cw20(Addr::unchecked("mock_token"));
        assert_eq!(AssetInfo::Cw20(Addr::unchecked("mock_token")), info);
    }

    #[test]
    fn native_asset_info_unchecked() {
        let info = AssetInfoUnchecked::Native("uusd".to_string());
        assert_eq!(AssetInfoUnchecked::Native("uusd".to_string()), info);
    }

    #[test]
    fn cw20_asset_info_unchecked() {
        let info = AssetInfoUnchecked::Cw20("mock_token".to_string());
        assert_eq!(AssetInfoUnchecked::Cw20("mock_token".to_string()), info);
    }

    #[test]
    #[cfg(feature = "astroport")]
    fn from_astro_asset_info() {
        let info = astroport::asset::AssetInfo::Token {
            contract_addr: Addr::unchecked("mock_token"),
        };
        let info2: AssetInfo = info.into();
        assert_eq!(info2, AssetInfo::Cw20(Addr::unchecked("mock_token")));

        let info = astroport::asset::AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        };
        let info2: AssetInfo = info.into();
        assert_eq!(info2, AssetInfo::Native("uusd".to_string()));
    }

    #[test]
    #[cfg(feature = "astroport")]
    fn into_astro_asset_info() {
        let info = AssetInfo::Cw20(Addr::unchecked("mock_token"));
        let info2: astroport::asset::AssetInfo = info.into();
        assert_eq!(
            info2,
            astroport::asset::AssetInfo::Token {
                contract_addr: Addr::unchecked("mock_token")
            }
        );

        let info = AssetInfo::Native("uusd".to_string());
        let info2: astroport::asset::AssetInfo = info.into();
        assert_eq!(
            info2,
            astroport::asset::AssetInfo::NativeToken {
                denom: "uusd".to_string()
            }
        );
    }
}
