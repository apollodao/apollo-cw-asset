use std::convert::{TryFrom, TryInto};
use std::fmt;

use cosmwasm_std::{
    from_binary, to_binary, Addr, Api, BankMsg, Binary, Coin, CosmosMsg, QuerierWrapper, StdError,
    StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20QueryMsg};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "terra")]
use {cosmwasm_std::QuerierWrapper, terra_cosmwasm::TerraQuerier};

use crate::AssetInfoUnchecked;

use super::asset_info::{AssetInfo, AssetInfoBase};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AssetBase<T> {
    pub info: AssetInfoBase<T>,
    pub amount: Uint128,
}

pub type AssetUnchecked = AssetBase<String>;
pub type Asset = AssetBase<Addr>;

impl From<Asset> for AssetUnchecked {
    fn from(asset: Asset) -> Self {
        AssetUnchecked {
            info: asset.info.into(),
            amount: asset.amount,
        }
    }
}

impl AssetUnchecked {
    pub fn new<A: Into<Uint128>>(info: AssetInfoUnchecked, amount: A) -> Self {
        Self {
            info,
            amount: amount.into(),
        }
    }

    pub fn native<A: Into<String>, B: Into<Uint128>>(denom: A, amount: B) -> Self {
        Self {
            info: AssetInfoUnchecked::native(denom),
            amount: amount.into(),
        }
    }

    pub fn cw20<A: Into<String>, B: Into<Uint128>>(contract_addr: A, amount: B) -> Self {
        Self {
            info: AssetInfoUnchecked::cw20(contract_addr),
            amount: amount.into(),
        }
    }

    /// Validate contract address (if any) and returns a new `Asset` instance
    pub fn check(&self, api: &dyn Api) -> StdResult<Asset> {
        Ok(Asset {
            info: self.info.check(api)?,
            amount: self.amount,
        })
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.info, self.amount)
    }
}

impl From<Coin> for Asset {
    fn from(coin: Coin) -> Self {
        Self {
            info: AssetInfo::Native(coin.denom),
            amount: coin.amount,
        }
    }
}

impl From<&Coin> for Asset {
    fn from(coin: &Coin) -> Self {
        coin.clone().into()
    }
}

impl TryFrom<Asset> for Coin {
    type Error = StdError;

    fn try_from(asset: Asset) -> Result<Self, Self::Error> {
        match asset.info {
            AssetInfo::Native(denom) => Ok(Self {
                denom,
                amount: asset.amount,
            }),
            _ => Err(StdError::parse_err(
                "Asset",
                "Cannot convert non-native asset to Coin",
            )),
        }
    }
}

impl TryInto<Coin> for &Asset {
    type Error = StdError;

    fn try_into(self) -> Result<Coin, Self::Error> {
        self.clone()
            .try_into()
            .map_err(|_| StdError::parse_err("Asset", "converting Asset to Coin"))
    }
}

impl From<Cw20Coin> for AssetUnchecked {
    fn from(coin: Cw20Coin) -> Self {
        Self {
            info: AssetInfoUnchecked::Cw20(coin.address),
            amount: coin.amount,
        }
    }
}

impl From<Cw20CoinVerified> for Asset {
    fn from(coin: Cw20CoinVerified) -> Self {
        Self {
            info: AssetInfo::Cw20(coin.address),
            amount: coin.amount,
        }
    }
}

impl TryFrom<Asset> for Cw20CoinVerified {
    type Error = StdError;

    fn try_from(asset: Asset) -> Result<Self, Self::Error> {
        match asset.info {
            AssetInfo::Cw20(contract_addr) => Ok(Self {
                address: contract_addr,
                amount: asset.amount,
            }),
            _ => Err(StdError::generic_err(
                "Cannot convert non-CW20 asset to Cw20Coin",
            )),
        }
    }
}

impl TryFrom<Asset> for Cw20Coin {
    type Error = StdError;

    fn try_from(asset: Asset) -> Result<Self, Self::Error> {
        let verified: Cw20CoinVerified = asset.try_into()?;
        Ok(Self {
            address: verified.address.to_string(),
            amount: verified.amount,
        })
    }
}

impl TryFrom<AssetUnchecked> for Cw20Coin {
    type Error = StdError;

    fn try_from(asset: AssetUnchecked) -> Result<Self, Self::Error> {
        match asset.info {
            AssetInfoUnchecked::Cw20(contract_addr) => Ok(Self {
                address: contract_addr,
                amount: asset.amount,
            }),
            _ => Err(StdError::generic_err(
                "Cannot convert non-CW20 asset to Cw20Coin",
            )),
        }
    }
}

#[cfg(feature = "astroport")]
impl From<astroport::asset::Asset> for Asset {
    fn from(asset: astroport::asset::Asset) -> Self {
        Self {
            info: asset.info.into(),
            amount: asset.amount,
        }
    }
}

#[cfg(feature = "astroport")]
impl From<Asset> for astroport::asset::Asset {
    fn from(asset: Asset) -> Self {
        Self {
            info: asset.info.into(),
            amount: asset.amount,
        }
    }
}

impl Asset {
    /// Create a new `AssetBase` instance based on given asset info and amount
    pub fn new<B: Into<Uint128>>(info: AssetInfo, amount: B) -> Self {
        Self {
            info,
            amount: amount.into(),
        }
    }

    /// Create a new `AssetBase` instance representing a CW20 token of given
    /// contract address and amount
    pub fn cw20<B: Into<Uint128>>(contract_addr: Addr, amount: B) -> Self {
        Self {
            info: AssetInfo::cw20(contract_addr),
            amount: amount.into(),
        }
    }

    /// Create a new `AssetBase` instance representing a native coin of given
    /// denom
    pub fn native<A: Into<String>, B: Into<Uint128>>(denom: A, amount: B) -> Self {
        Self {
            info: AssetInfo::native(denom),
            amount: amount.into(),
        }
    }

    /// Generate a message that sends a CW20 token to the specified recipient
    /// with a binary payload
    ///
    /// NOTE: Only works for CW20 tokens
    ///
    /// **Usage:**
    /// The following code generates a message that sends 12345 units of a mock
    /// token to a contract, invoking a mock execute function.
    ///
    /// ```rust
    /// let asset = Asset::cw20(Addr::unchecked("mock_token"), 12345);
    /// let msg = asset.send_msg("mock_contract", to_binary(&ExecuteMsg::MockFunction {})?)?;
    /// ```
    pub fn send_msg<A: Into<String>>(&self, to: A, msg: Binary) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: to.into(),
                    amount: self.amount,
                    msg,
                })?,
                funds: vec![],
            })),
            AssetInfo::Native(_) => Err(StdError::generic_err(
                "native coins do not have `send` method",
            )),
        }
    }

    /// Generate a message that transfers the asset from the sender to account
    /// `to`
    ///
    /// NOTE: It is generally neccessary to first deduct tax before calling this
    /// method.
    ///
    /// **Usage:**
    /// The following code generates a message that sends 12345 uusd (i.e.
    /// 0.012345 UST) to Alice. Note that due to tax, the actual deliverable
    /// amount is smaller than 12345 uusd.
    ///
    /// ```rust
    /// let asset = Asset::native("uusd", 12345);
    /// let msg = asset.deduct_tax(&deps.querier)?.transfer_msg("alice")?;
    /// ```
    pub fn transfer_msg<A: Into<String>>(&self, to: A) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: to.into(),
                    amount: self.amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::Native(denom) => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: to.into(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: self.amount,
                }],
            })),
        }
    }

    /// Generate a message that draws the asset from account `from` to account
    /// `to`
    ///
    /// **Usage:**
    /// The following code generates a message that draws 69420 uMIR token from
    /// Alice's wallet to Bob's. Note that Alice must have approve this
    /// spending for this transaction to work.
    ///
    /// ```rust
    /// let asset = Asset::cw20("mirror_token", 69420);
    /// let msg = asset.transfer_from_msg("alice", "bob")?;
    /// ```
    pub fn transfer_from_msg<A: Into<String>, B: Into<String>>(
        &self,
        from: A,
        to: B,
    ) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: from.into(),
                    recipient: to.into(),
                    amount: self.amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::Native(_) => Err(StdError::generic_err(
                "native coins do not have `transfer_from` method",
            )),
        }
    }

    /// Query balance of the asset for the given address
    pub fn query_balance(&self, querier: &QuerierWrapper, addr: &Addr) -> StdResult<Uint128> {
        match &self.info {
            AssetInfo::Cw20(contract_addr) => {
                let res: cw20::BalanceResponse = from_binary(&querier.query_wasm_smart(
                    contract_addr.as_str(),
                    &Cw20QueryMsg::Balance {
                        address: addr.to_string(),
                    },
                )?)?;
                Ok(res.balance)
            }
            AssetInfo::Native(denom) => querier
                .query_balance(addr.as_str(), denom.as_str())
                .map(|c| c.amount),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockApi;

    use test_case::test_case;

    fn apollo() -> Asset {
        Asset {
            info: AssetInfo::Cw20(Addr::unchecked("apollo")),
            amount: Uint128::new(123456u128),
        }
    }

    fn uusd() -> Asset {
        Asset {
            info: AssetInfo::Native(String::from("uusd")),
            amount: Uint128::new(123456u128),
        }
    }

    #[derive(Serialize)]
    enum MockExecuteMsg {
        MockCommand {},
    }

    #[test]
    fn creating_instances() {
        let info = AssetInfo::Native(String::from("uusd"));
        let asset = Asset::new(info, 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        );

        let asset = Asset::cw20(Addr::unchecked("mock_token"), 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("mock_token")),
                amount: Uint128::new(123456u128)
            }
        );

        let asset = Asset::native("uusd", 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        )
    }

    #[test]
    fn comparing() {
        let uluna1 = Asset::native("uluna", 69u128);
        let uluna2 = Asset::native("uluna", 420u128);
        let uusd = Asset::native("uusd", 69u128);
        let astro = Asset::cw20(Addr::unchecked("astro_token"), 69u128);

        assert!(uluna1 != uluna2);
        assert!(uluna1 != uusd);
        assert!(astro == astro.clone());
    }

    #[test]
    fn displaying() {
        let asset = Asset::native("uusd", 69420u128);
        assert_eq!(asset.to_string(), String::from("uusd:69420"));

        let asset = Asset::cw20(Addr::unchecked("mock_token"), 88888u128);
        assert_eq!(asset.to_string(), String::from("mock_token:88888"));
    }

    #[test]
    fn casting() {
        let api = MockApi::default();

        let checked = Asset::cw20(Addr::unchecked("mock_token"), 123456u128);
        let unchecked: AssetUnchecked = checked.clone().into();

        assert_eq!(unchecked.check(&api).unwrap(), checked);
    }

    #[test]
    fn creating_messages() {
        let token = Asset::cw20(Addr::unchecked("mock_token"), 123456u128);
        let coin = Asset::native("uusd", 123456u128);

        let bin_msg = to_binary(&MockExecuteMsg::MockCommand {}).unwrap();
        let msg = token.send_msg("mock_contract", bin_msg.clone()).unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("mock_token"),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: String::from("mock_contract"),
                    amount: Uint128::new(123456),
                    msg: to_binary(&MockExecuteMsg::MockCommand {}).unwrap()
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let err = coin.send_msg("mock_contract", bin_msg);
        assert_eq!(
            err,
            Err(StdError::generic_err(
                "native coins do not have `send` method"
            ))
        );

        let msg = token.transfer_msg("alice").unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("mock_token"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::new(123456)
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let msg = coin.transfer_msg("alice").unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("alice"),
                amount: vec![Coin::new(123456, "uusd")]
            })
        );

        let msg = token.transfer_from_msg("bob", "charlie").unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("mock_token"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: String::from("bob"),
                    recipient: String::from("charlie"),
                    amount: Uint128::new(123456)
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let err = coin.transfer_from_msg("bob", "charlie");
        assert_eq!(
            err,
            Err(StdError::generic_err(
                "native coins do not have `transfer_from` method"
            ))
        );
    }

    #[test]
    fn new() {
        let asset = Asset::new(AssetInfo::Native(String::from("uusd")), 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        );
    }

    #[test]
    fn native() {
        let asset = Asset::native("uusd", 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        );
    }

    #[test]
    fn cw20() {
        let asset = Asset::cw20(Addr::unchecked("mock_token"), 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("mock_token")),
                amount: Uint128::new(123456u128)
            }
        );
    }

    #[test]
    fn new_unchecked() {
        let asset =
            AssetUnchecked::new(AssetInfoUnchecked::Native(String::from("uusd")), 123456u128);
        assert_eq!(
            asset,
            AssetUnchecked {
                info: AssetInfoUnchecked::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        );
    }

    #[test]
    fn native_unchecked() {
        let asset = AssetUnchecked::native("uusd", 123456u128);
        assert_eq!(
            asset,
            AssetUnchecked {
                info: AssetInfoUnchecked::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        );
    }

    #[test]
    fn cw20_unchecked() {
        let asset = AssetUnchecked::cw20("mock_token", 123456u128);
        assert_eq!(
            asset,
            AssetUnchecked {
                info: AssetInfoUnchecked::Cw20("mock_token".to_string()),
                amount: Uint128::new(123456u128)
            }
        );
    }

    #[test]
    fn from_cw20coin_for_assetunchecked() {
        let coin = Cw20Coin {
            address: "mock_token".to_string(),
            amount: Uint128::new(123456u128),
        };
        assert_eq!(
            AssetUnchecked::cw20("mock_token".to_string(), 123456u128),
            AssetUnchecked::from(coin)
        );
    }

    #[test]
    fn from_cw20coinverified_for_asset() {
        let coin = Cw20CoinVerified {
            address: Addr::unchecked("apollo"),
            amount: Uint128::new(123456u128),
        };
        assert_eq!(apollo(), Asset::from(coin));
    }

    #[test_case(uusd() => matches Err(_) ; "native")]
    #[test_case(apollo() => Ok(Cw20CoinVerified {
                    address: Addr::unchecked("apollo"),
                    amount: 123456u128.into()
                }) ; "cw20")]
    fn try_from_asset_for_cw20coinverified(asset: Asset) -> StdResult<Cw20CoinVerified> {
        Cw20CoinVerified::try_from(asset)
    }

    #[test_case(uusd() => matches Err(_) ; "native")]
    #[test_case(apollo() => Ok(Cw20Coin {
                    address: "apollo".to_string(),
                    amount: 123456u128.into()
                }) ; "cw20")]
    fn try_from_asset_for_cw20coin(asset: Asset) -> StdResult<Cw20Coin> {
        Cw20Coin::try_from(asset)
    }

    #[test_case(uusd().into() => matches Err(_) ; "native")]
    #[test_case(apollo().into() => Ok(Cw20Coin {
                    address: "apollo".to_string(),
                    amount: 123456u128.into()
                }) ; "cw20")]
    fn try_from_assetunchecked_for_cw20coin(asset: AssetUnchecked) -> StdResult<Cw20Coin> {
        Cw20Coin::try_from(asset)
    }
}
