use std::fmt::Formatter;
use std::{convert::TryFrom, fmt};

use cosmwasm_std::{
    to_binary, Addr, Api, BalanceResponse, BankQuery, QuerierWrapper, QueryRequest, StdError,
    StdResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};

use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
        AssetInfoKey {
            bytes,
        }
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
        self.to_owned() == AssetInfoKey::from(other)
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
        Ok(Self {
            bytes: value,
        })
    }
}

impl<'a> Prefixer<'a> for AssetInfoKey {
    fn prefix(&self) -> Vec<cw_storage_plus::Key> {
        vec![Key::Ref(&self.bytes)]
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
}
