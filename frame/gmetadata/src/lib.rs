#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod datastructures;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

    use crate::datastructures::*;
    use frame_support::{ensure, pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_namespace_id)]
    pub type NamespaceIdStore<T: Config> =
        StorageMap<_, Blake2_128Concat, GmetadataNamespaceName, u64>;

    #[pallet::storage]
    #[pallet::getter(fn get_namespace)]
    pub type NamespaceStore<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, GmetadataNamespaceInfo<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn latest_namespace_id)]
    pub type LatestNamespaceId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_value)]
    pub type ValueStore<T: Config> =
        StorageMap<_, Blake2_128Concat, GmetadataKey, GmetadataValueInfo>;

    #[pallet::storage]
    #[pallet::getter(fn get_index)]
    pub type IndexStore<T: Config> =
        StorageMap<_, Blake2_128Concat, GmetadataKey, GmetadataIndexInfo>;

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        NamespaceAlreadyExist,
        NamespaceNotFound,
        NamespaceOwnerAlreadyExists,
        InvalidNamespaceName,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn namespace_add_owner(
            origin: OriginFor<T>,
            id: u64,
            account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            NamespaceStore::<T>::try_mutate(id, |ns| -> DispatchResult {
                match ns {
                    Some(ns) => {
                        Self::check_owner_or_root(&origin, &ns.owners)?;
                        if !ns.owners.contains(&account) {
                            ns.owners.push(account);
                            Ok(())
                        } else {
                            Err(Error::<T>::NamespaceOwnerAlreadyExists.into())
                        }
                    }
                    None => Err(Error::<T>::NamespaceNotFound.into()),
                }
            })?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn namespace_remove_owner(
            origin: OriginFor<T>,
            id: u64,
            account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let mut ns = match NamespaceStore::<T>::get(id) {
                Some(ns) => ns,
                None => return Err(Error::<T>::NamespaceNotFound.into()),
            };
            Self::check_owner_or_root(&origin, &ns.owners)?;
            match ns.owners.iter().position(|a| a.eq(&account)) {
                Some(idx) => {
                    ns.owners.remove(idx);
                    NamespaceStore::<T>::insert(id, ns);
                }
                None => {}
            };
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn create_namespace(
            origin: OriginFor<T>,
            name: GmetadataNamespaceName,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin.clone())?;
            if name.len() == 0 {
                return Err(Error::<T>::InvalidNamespaceName.into());
            }

            let namespace_id = NamespaceIdStore::<T>::get(&name);
            if namespace_id.is_some() {
                return Err(Error::<T>::NamespaceAlreadyExist.into());
            }
            let namespace_id = Self::latest_namespace_id().saturating_add(1);
            let owners = Vec::<T::AccountId>::new();
            NamespaceStore::<T>::insert(
                namespace_id,
                GmetadataNamespaceInfo {
                    id: namespace_id,
                    name: name.clone(),
                    owners,
                },
            );

            NamespaceIdStore::<T>::insert(name, namespace_id);
            LatestNamespaceId::<T>::put(namespace_id);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn batch_write(
            origin: OriginFor<T>,
            ops: Vec<GmetadataWriteOp>,
        ) -> DispatchResultWithPostInfo {
            for op in ops {
                match op {
                    GmetadataWriteOp::SetValue(key, value) => {
                        Self::set_value(origin.clone(), key, value)?;
                    }
                    GmetadataWriteOp::RemoveValue(key) => {
                        Self::remove_value(origin.clone(), key)?;
                    }
                    GmetadataWriteOp::AddIndex(key, value) => {
                        Self::add_index(origin.clone(), key, value)?;
                    }
                    GmetadataWriteOp::RemoveIndex(key, value) => {
                        Self::remove_index(origin.clone(), key, value)?;
                    }
                }
            }
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn set_value(
            origin: OriginFor<T>,
            key: GmetadataKey,
            value: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            ValueStore::<T>::insert(
                key,
                GmetadataValueInfo {
                    data: value,
                    update_time: 0,
                },
            );
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn add_index(
            origin: OriginFor<T>,
            key: GmetadataKey,
            value: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            let mut old_value = IndexStore::<T>::get(&key);
            match &mut old_value {
                Some(old_value) => {
                    if !old_value.data.contains(&value) {
                        old_value.data.push(value);
                        old_value.data.sort();
                        old_value.update_time = 0;
                        IndexStore::<T>::insert(key, old_value);
                    }
                }
                None => IndexStore::<T>::insert(
                    key,
                    GmetadataIndexInfo {
                        data: sp_std::vec![value],
                        update_time: 0,
                    },
                ),
            }
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn remove_value(origin: OriginFor<T>, key: GmetadataKey) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            ValueStore::<T>::remove(key);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn remove_index(
            origin: OriginFor<T>,
            key: GmetadataKey,
            value: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            Self::check_namespace(origin, key.ns)?;
            let mut old_value = IndexStore::<T>::get(&key);
            match &mut old_value {
                Some(old_value) => match old_value.data.iter().position(|v| v == &value) {
                    Some(idx) => {
                        old_value.data.remove(idx);
                        old_value.data.sort();
                        old_value.update_time = 0;
                        IndexStore::<T>::insert(key, old_value);
                    }
                    None => {}
                },
                None => {}
            }
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn value(key: GmetadataKey) -> Option<Vec<u8>> {
            match ValueStore::<T>::get(key) {
                Some(value) => Some(value.data),
                None => None,
            }
        }

        fn check_owner_or_root(
            origin: &OriginFor<T>,
            owners: &Vec<T::AccountId>,
        ) -> DispatchResult {
            match ensure_signed(origin.clone()) {
                Ok(who) => {
                    if !owners.contains(&who) {
                        return Err(DispatchError::BadOrigin);
                    }
                }
                Err(_) => ensure_root(origin.clone())?,
            }
            Ok(().into())
        }

        fn check_namespace(origin: OriginFor<T>, ns_id: u64) -> DispatchResultWithPostInfo {
            let ns = match NamespaceStore::<T>::get(ns_id) {
                Some(ns) => ns,
                None => return Err(Error::<T>::NamespaceNotFound.into()),
            };
            Self::check_owner_or_root(&origin, &ns.owners)?;
            Ok(().into())
        }

        pub fn query_with_index(
            index_key: GmetadataKey,
            mut value_key: GmetadataKey,
            start: Option<Vec<u8>>,
            limit: usize,
        ) -> GmetadataQueryResult {
            match Self::get_index(index_key) {
                Some(index_info) => {
                    let mut result = Vec::new();
                    let mut cursor = None;
                    let mut skip = true;
                    for key in &index_info.data {
                        if skip {
                            match &start {
                                Some(n) => {
                                    if n.len() == 0 {
                                        skip = false;
                                    } else if key.eq(n) {
                                        skip = false;
                                        continue;
                                    }
                                }
                                None => skip = false,
                            }
                        }
                        if skip {
                            continue;
                        }
                        value_key.pk = key.clone();
                        match Self::get_value(value_key.clone()) {
                            Some(val) => {
                                result.push(val.data);
                                if result.len() >= limit {
                                    if Some(key) != index_info.data.last() {
                                        cursor = Some(key.clone());
                                    }
                                    break;
                                }
                            }
                            None => {}
                        };
                    }
                    let cursor = match cursor {
                        Some(cursor) => cursor,
                        None => Vec::new(),
                    };
                    GmetadataQueryResult {
                        list: result,
                        cursor,
                    }
                }
                None => GmetadataQueryResult::default(),
            }
        }
    }
}
