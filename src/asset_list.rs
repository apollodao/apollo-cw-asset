use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::slice::{Iter, IterMut};

use cosmwasm_std::{Addr, Api, Coin, CosmosMsg, QuerierWrapper, StdError, StdResult};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::AssetUnchecked;

use super::asset::{Asset, AssetBase};
use super::asset_info::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AssetListBase<T>(pub(crate) Vec<AssetBase<T>>);

#[allow(clippy::derivable_impls)] // clippy says `Default` can be derived here, but actually it can't
impl<T> Default for AssetListBase<T> {
    fn default() -> Self {
        Self(vec![])
    }
}

pub type AssetListUnchecked = AssetListBase<String>;
pub type AssetList = AssetListBase<Addr>;

#[cfg(feature = "astroport")]
impl From<AssetList> for Vec<astroport::asset::Asset> {
    fn from(value: AssetList) -> Self {
        value
            .0
            .into_iter()
            .map(|asset| asset.into())
            .collect::<Vec<astroport::asset::Asset>>()
    }
}

impl From<Vec<AssetUnchecked>> for AssetListUnchecked {
    fn from(assets: Vec<AssetUnchecked>) -> Self {
        Self(assets)
    }
}

impl From<AssetList> for AssetListUnchecked {
    fn from(list: AssetList) -> Self {
        Self(
            list.to_vec()
                .iter()
                .cloned()
                .map(|asset| asset.into())
                .collect(),
        )
    }
}

impl<A, B> From<B> for AssetList
where
    A: Into<Asset>,
    B: IntoIterator<Item = A>,
{
    fn from(list: B) -> Self {
        let mut asset_list = AssetList::default();
        for asset in list {
            asset_list.add(&asset.into()).unwrap();
        }
        asset_list
    }
}

impl<A> TryFrom<AssetList> for [A; 2]
where
    A: From<Asset>,
{
    type Error = StdError;

    fn try_from(value: AssetList) -> Result<[A; 2], Self::Error> {
        if value.len() != 2 {
            return Err(StdError::generic_err(format!(
                "AssetList must contain exactly 2 assets, but it contains {}",
                value.len()
            )));
        }
        let other_assets = value.to_vec();
        Ok([
            other_assets[0].to_owned().into(),
            other_assets[1].to_owned().into(),
        ])
    }
}

impl From<AssetList> for Vec<Asset> {
    fn from(list: AssetList) -> Self {
        list.0
    }
}

impl TryFrom<AssetList> for Vec<Coin> {
    type Error = StdError;

    fn try_from(list: AssetList) -> StdResult<Self> {
        list.0
            .into_iter()
            .map(|asset| asset.try_into())
            .collect::<StdResult<Vec<Coin>>>()
    }
}

impl AssetListUnchecked {
    /// Validate contract address of every asset in the list, and return a new
    /// `AssetList` instance
    pub fn check(&self, api: &dyn Api) -> StdResult<AssetList> {
        let mut assets = AssetList::default();
        for asset in &self.0 {
            assets.add(&asset.check(api)?)?;
        }
        Ok(assets)
    }
}

impl fmt::Display for AssetList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|asset| asset.to_string())
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}

impl<'a> IntoIterator for &'a AssetList {
    type Item = &'a Asset;
    type IntoIter = std::slice::Iter<'a, Asset>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl AssetList {
    /// Create a new, empty asset list
    pub fn new() -> Self {
        AssetListBase::default()
    }

    /// Return a copy of the underlying vector
    pub fn to_vec(&self) -> Vec<Asset> {
        self.0.to_vec()
    }

    /// Return length of the asset list
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns an iterator over the asset list
    pub fn iter(&self) -> Iter<Asset> {
        self.0.iter()
    }

    /// Returns a mutable iterator over the asset list
    pub fn iter_mut(&mut self) -> IterMut<Asset> {
        self.0.iter_mut()
    }

    /// Returns a reference to the asset at the given index. Return `None` if
    /// the index does not exist.
    pub fn get(&self, idx: usize) -> Option<&Asset> {
        self.0.get(idx)
    }

    /// Returns a vector of all native coins in the asset list.
    pub fn get_native_coins(&self) -> Vec<Coin> {
        self.iter()
            .filter_map(|a| {
                let native: StdResult<Coin> = a.try_into();
                if let Ok(coin) = native {
                    Some(coin)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find an asset in the list that matches the provided asset info
    ///
    /// Return `Some(&asset)` if found, where `&asset` is a reference to the
    /// asset found; `None` if not found.
    pub fn find(&self, info: &AssetInfo) -> Option<&Asset> {
        self.0.iter().find(|asset| asset.info == *info)
    }

    /// Apply a mutation on each of the asset
    pub fn apply<F: FnMut(&mut Asset)>(&mut self, f: F) -> &mut Self {
        self.0.iter_mut().for_each(f);
        self
    }

    /// Removes all assets in the list that has zero amount
    pub fn purge(&mut self) -> &mut Self {
        self.0.retain(|asset| !asset.amount.is_zero());
        self
    }

    /// Add a new asset to the list
    ///
    /// If asset of the same kind already exists in the list, then increment its
    /// amount; if not, append to the end of the list.
    pub fn add(&mut self, asset_to_add: &Asset) -> StdResult<&mut Self> {
        match self
            .0
            .iter_mut()
            .find(|asset| asset.info == asset_to_add.info)
        {
            Some(asset) => {
                asset.amount = asset.amount.checked_add(asset_to_add.amount)?;
            }
            None => {
                self.0.push(asset_to_add.clone());
            }
        }
        Ok(self.purge())
    }

    /// Add multiple new assets to the list
    pub fn add_many(&mut self, assets_to_add: &AssetList) -> StdResult<&mut Self> {
        for asset in &assets_to_add.0 {
            self.add(asset)?;
        }
        Ok(self)
    }

    /// Deduct an asset from the list
    ///
    /// The asset of the same kind and equal or greater amount must already
    /// exist in the list. If so, deduct the amount from the asset; ifnot,
    /// throw an error.
    ///
    /// If an asset's amount is reduced to zero, it is purged from the list.
    pub fn deduct(&mut self, asset_to_deduct: &Asset) -> StdResult<&mut Self> {
        match self
            .0
            .iter_mut()
            .find(|asset| asset.info == asset_to_deduct.info)
        {
            Some(asset) => {
                asset.amount = asset.amount.checked_sub(asset_to_deduct.amount)?;
            }
            None => {
                return Err(StdError::generic_err(format!(
                    "not found: {}",
                    asset_to_deduct.info
                )))
            }
        }
        Ok(self.purge())
    }

    /// Deduct multiple assets from the list
    pub fn deduct_many(&mut self, assets_to_deduct: &AssetList) -> StdResult<&mut Self> {
        for asset in &assets_to_deduct.0 {
            self.deduct(asset)?;
        }
        Ok(self)
    }

    /// Generate a transfer messages for every asset in the list
    pub fn transfer_msgs<A: Into<String> + Clone>(&self, to: A) -> StdResult<Vec<CosmosMsg>> {
        self.0
            .iter()
            .map(|asset| asset.transfer_msg(to.clone()))
            .collect::<StdResult<Vec<CosmosMsg>>>()
    }

    /// Query balances for all assets in the list for the given address and
    /// return a new `AssetList`
    pub fn query_balances(&self, querier: &QuerierWrapper, addr: &Addr) -> StdResult<AssetList> {
        self.into_iter()
            .map(|asset| {
                Ok(Asset::new(
                    asset.info.clone(),
                    asset.query_balance(querier, addr)?,
                ))
            })
            .collect::<StdResult<Vec<Asset>>>()
            .map(Into::into)
    }

    /// Queries balances for all `AssetInfo` objects in the given vec for the
    /// given address and return a new `AssetList`
    pub fn query_asset_info_balances(
        asset_infos: Vec<AssetInfo>,
        querier: &QuerierWrapper,
        addr: &Addr,
    ) -> StdResult<AssetList> {
        asset_infos
            .into_iter()
            .map(|asset_info| {
                Ok(Asset::new(
                    asset_info.clone(),
                    asset_info.query_balance(querier, addr)?,
                ))
            })
            .collect::<StdResult<Vec<Asset>>>()
            .map(Into::into)
    }
}

#[cfg(test)]
mod test_helpers {
    use super::super::asset::Asset;
    use super::*;

    pub fn uluna() -> AssetInfo {
        AssetInfo::native("uluna")
    }

    pub fn uusd() -> AssetInfo {
        AssetInfo::native("uusd")
    }

    pub fn mock_token() -> AssetInfo {
        AssetInfo::cw20(Addr::unchecked("mock_token"))
    }

    pub fn mock_list() -> AssetList {
        AssetList::from(vec![
            Asset::native("uusd", 69420u128),
            Asset::new(mock_token(), 88888u128),
        ])
    }

    #[cfg(feature = "astroport")]
    pub fn mock_astro_list() -> Vec<astroport::asset::Asset> {
        use cosmwasm_std::Uint128;

        vec![
            astroport::asset::Asset {
                info: astroport::asset::AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(69420u128),
            },
            astroport::asset::Asset {
                info: astroport::asset::AssetInfo::Token {
                    contract_addr: Addr::unchecked("mock_token"),
                },
                amount: Uint128::from(88888u128),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::{AssetInfoUnchecked, AssetUnchecked as AU};

    use super::super::asset::Asset;
    use super::test_helpers::{mock_list, mock_token, uluna, uusd};
    use super::*;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::{
        to_binary, BankMsg, Coin, CosmosMsg, Decimal, OverflowError, OverflowOperation, Uint128,
        WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;

    use test_case::test_case;

    #[test]
    fn displaying() {
        let list = mock_list();
        assert_eq!(
            list.to_string(),
            String::from("uusd:69420,mock_token:88888")
        );
    }

    #[test]
    fn casting() {
        let api = MockApi::default();

        let checked = mock_list();
        let unchecked: AssetListUnchecked = checked.clone().into();

        assert_eq!(unchecked.check(&api).unwrap(), checked);
    }

    #[test]
    fn finding() {
        let list = mock_list();

        let asset_option = list.find(&uusd());
        assert_eq!(asset_option, Some(&Asset::new(uusd(), 69420u128)));

        let asset_option = list.find(&mock_token());
        assert_eq!(asset_option, Some(&Asset::new(mock_token(), 88888u128)));
    }

    #[test]
    fn applying() {
        let mut list = mock_list();

        let half = Decimal::from_ratio(1u128, 2u128);
        list.apply(|asset: &mut Asset| asset.amount = asset.amount * half);
        assert_eq!(
            list,
            AssetList::from(vec![
                Asset::native("uusd", 34710u128),
                Asset::new(mock_token(), 44444u128)
            ])
        );
    }

    #[test]
    fn adding() {
        let mut list = mock_list();

        list.add(&Asset::new(uluna(), 12345u128)).unwrap();
        let asset = list.find(&uluna()).unwrap();
        assert_eq!(asset.amount, Uint128::new(12345));

        list.add(&Asset::new(uusd(), 1u128)).unwrap();
        let asset = list.find(&uusd()).unwrap();
        assert_eq!(asset.amount, Uint128::new(69421));
    }

    #[test]
    fn adding_many() {
        let mut list = mock_list();
        list.add_many(&mock_list()).unwrap();

        let expected = mock_list().apply(|a| a.amount *= Uint128::new(2)).clone();
        assert_eq!(list, expected);
    }

    #[test]
    fn deducting() {
        let mut list = mock_list();

        list.deduct(&Asset::new(uusd(), 12345u128)).unwrap();
        let asset = list.find(&uusd()).unwrap();
        assert_eq!(asset.amount, Uint128::new(57075));

        list.deduct(&Asset::new(uusd(), 57075u128)).unwrap();
        let asset_option = list.find(&uusd());
        assert_eq!(asset_option, None);

        let err = list.deduct(&Asset::new(uusd(), 57075u128));
        assert_eq!(err, Err(StdError::generic_err("not found: uusd")));

        list.deduct(&Asset::new(mock_token(), 12345u128)).unwrap();
        let asset = list.find(&mock_token()).unwrap();
        assert_eq!(asset.amount, Uint128::new(76543));

        let err = list.deduct(&Asset::new(mock_token(), 99999u128));
        assert_eq!(
            err,
            Err(StdError::overflow(OverflowError::new(
                OverflowOperation::Sub,
                Uint128::new(76543),
                Uint128::new(99999)
            )))
        );
    }

    #[test]
    fn deducting_many() {
        let mut list = mock_list();
        list.deduct_many(&mock_list()).unwrap();
        assert_eq!(list, AssetList::new());
    }

    #[test]
    fn creating_messages() {
        let list = mock_list();
        let msgs = list.transfer_msgs("alice").unwrap();
        assert_eq!(
            msgs,
            vec![
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: String::from("alice"),
                    amount: vec![Coin::new(69420, "uusd")]
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mock_token"),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: String::from("alice"),
                        amount: Uint128::new(88888)
                    })
                    .unwrap(),
                    funds: vec![]
                })
            ]
        );
    }

    #[test]
    fn unchecked_from_vec() {
        let asset1 = AssetUnchecked {
            info: AssetInfoUnchecked::Native("token1".to_string()),
            amount: Uint128::new(12345),
        };
        let asset2 = AssetUnchecked {
            info: AssetInfoUnchecked::Native("token2".to_string()),
            amount: Uint128::new(67890),
        };
        let api = MockApi::default();

        let list: AssetListUnchecked = vec![asset1.clone(), asset2.clone()].into();

        let expected = AssetList::from(vec![
            asset1.check(&api).unwrap(),
            asset2.check(&api).unwrap(),
        ]);

        assert_eq!(list.check(&api).unwrap(), expected);
    }

    #[test]
    fn generic_from() {
        let coins = vec![Coin::new(1234, "coin1"), Coin::new(5678, "coin2")];

        let list: AssetList = coins.into();

        let unchecked = AssetListUnchecked::from(vec![
            AssetUnchecked {
                info: AssetInfoUnchecked::Native("coin1".to_string()),
                amount: Uint128::new(1234),
            },
            AssetUnchecked {
                info: AssetInfoUnchecked::Native("coin2".to_string()),
                amount: Uint128::new(5678),
            },
        ]);

        assert_eq!(list, unchecked.check(&MockApi::default()).unwrap());
    }

    #[test_case(vec![], vec![]; "empty")]
    #[test_case(vec![AU::native("coin1", 12345u128), AU::native("coin2", 67890u128)],
                vec![Asset::native("coin1", 12345u128), Asset::native("coin2", 67890u128)];
                "native")]
    #[test_case(vec![AU::native("coin1", 12345u128), AU::native("coin1", 67890u128)],
                vec![Asset::native("coin1", 80235u128)] ;
                "duplicates")]
    #[test_case(vec![AU::native("coin1", 12345u128), AU::cw20("coin2", 67890u128)],
                vec![Asset::native("coin1", 12345u128), Asset::cw20(Addr::unchecked("coin2"), 67890u128)];
                "cw20 valid mock address")]
    #[test_case(vec![AU::native("coin1", 12345u128), AU::cw20("co", 67890u128)],
                vec![Asset::native("coin1", 12345u128), Asset::cw20(Addr::unchecked("co"), 67890u128)]
                => matches Err(_) ;
                "cw20 invalid mock address")]
    fn check(unchecked: Vec<AssetUnchecked>, expected: Vec<Asset>) -> StdResult<()> {
        let unchecked = AssetListUnchecked::from(unchecked);

        assert_eq!(
            unchecked.check(&MockApi::default())?,
            AssetList::from(expected)
        );

        Ok(())
    }

    #[test]
    fn into_iter() {
        let list = mock_list();
        let mut iter = (&list).into_iter();
        assert_eq!(iter.next(), Some(&Asset::new(uusd(), 69420u128)));
        assert_eq!(iter.next(), Some(&Asset::new(mock_token(), 88888u128)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter() {
        let list = mock_list();
        let mut iter = list.iter();
        assert_eq!(iter.next(), Some(&Asset::new(uusd(), 69420u128)));
        assert_eq!(iter.next(), Some(&Asset::new(mock_token(), 88888u128)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_mut() {
        let mut list = mock_list();
        let mut iter = list.iter_mut();
        assert_eq!(iter.next(), Some(&mut Asset::new(uusd(), 69420u128)));
        assert_eq!(iter.next(), Some(&mut Asset::new(mock_token(), 88888u128)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn get() {
        let list = mock_list();
        assert_eq!(list.get(0), Some(&Asset::new(uusd(), 69420u128)));
        assert_eq!(list.get(1), Some(&Asset::new(mock_token(), 88888u128)));
        assert_eq!(list.get(2), None);
    }

    #[test]
    fn get_native_coins() {
        let list = mock_list();
        assert_eq!(list.get_native_coins(), vec![Coin::new(69420, "uusd")]);
    }

    #[test]
    fn from_assetlist_for_vec_asset() {
        let list = mock_list();

        let vec_asset = Vec::<Asset>::from(list);

        assert_eq!(
            vec_asset,
            vec![
                Asset::native("uusd", 69420u128),
                Asset::cw20(Addr::unchecked("mock_token"), 88888u128)
            ]
        );
    }

    #[test]
    #[cfg(feature = "astroport")]
    fn from_assetlist_for_vec_astro_asset_info() {
        use crate::asset_list::test_helpers::mock_astro_list;

        let list = mock_list();

        let vec_asset_info = Vec::<astroport::asset::Asset>::from(list);

        assert_eq!(vec_asset_info, mock_astro_list());
    }
}
