#![cfg_attr(not(feature = "std"), no_std)]
// #![feature(map_first_last)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use automata_traits::attestor::{ApplicationTrait, AttestorTrait};
    use core::convert::{TryFrom, TryInto};
    use frame_support::ensure;
    use frame_support::traits::{Currency, ReservableCurrency};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo, pallet_prelude::*, unsigned::ValidateUnsigned,
    };
    use frame_system::ensure_signed;
    use frame_system::pallet_prelude::OriginFor;
    use frame_system::RawOrigin;
    use frame_system::{
        offchain::{SendTransactionTypes, SubmitTransaction},
        pallet_prelude::*,
    };
    use primitives::{attestor::ReportType, BlockNumber};
    #[cfg(feature = "full_crypto")]
    use sp_core::crypto::Pair;
    #[cfg(feature = "full_crypto")]
    use sp_core::sr25519::Pair as Sr25519Pair;
    use sp_core::sr25519::{Public, Signature};
    use sp_runtime::{Percent, RuntimeDebug, SaturatedConversion};
    use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
    use sp_std::prelude::*;

    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};

    /// Attestor struct
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Attestor<AccountId: Ord> {
        /// Attestor's url, geode will get it and communicate with attestor.
        pub url: Vec<u8>,
        /// Attestor's Secp256r1PublicKey
        pub pubkey: Vec<u8>,
        /// Geode being attested by this attestor
        pub applications: BTreeSet<AccountId>,
    }

    /// The geode struct shows its status
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Report<AccountId: Ord> {
        pub start: BlockNumber,
        pub attestors: BTreeSet<AccountId>,
    }

    pub type ReportOf<T> = Report<<T as frame_system::Config>::AccountId>;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    pub type AttestorOf<T> = Attestor<<T as frame_system::Config>::AccountId>;

    pub const UNSIGNED_TXS_PRIORITY: u64 = 100;
    pub const DEFAULT_ATT_STAKE_MIN: primitives::Balance = 1000;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: SendTransactionTypes<Call<Self>> + frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency in which fees are paid and contract balances are held.
        type Currency: ReservableCurrency<Self::AccountId>;

        type Call: From<Call<Self>>;

        #[pallet::constant]
        type HeartbeatTimeoutBlockNumber: Get<u32>;

        type ApplicationHandler: ApplicationTrait<AccountId = Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn minimum_attestor_num)]
    pub type MinimumAttestorNum<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultMinimumAttestorNum<T>>;

    #[pallet::type_value]
    pub fn DefaultMinimumAttestorNum<T: Config>() -> u16 {
        1
    }

    #[pallet::storage]
    #[pallet::getter(fn expected_attestor_num)]
    pub type ExpectedAttestorNum<T: Config> =
        StorageValue<_, u16, ValueQuery, DefaultExpectedAttestorNum<T>>;

    #[pallet::type_value]
    pub fn DefaultExpectedAttestorNum<T: Config>() -> u16 {
        2
    }

    // The pallet's runtime storage items.
    #[pallet::storage]
    #[pallet::getter(fn attestors)]
    pub type Attestors<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AttestorOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn attestors_whitelist)]
    pub type AttestorsWhiteList<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn geode_attestors)]
    pub type AttestorsOfApplications<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BTreeSet<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn attestor_last_notification)]
    pub type AttestorLastNotify<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumber, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn reports)]
    pub(super) type Reports<T: Config> =
        StorageMap<_, Blake2_128Concat, (T::AccountId, u8), ReportOf<T>, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultAttestorNum<T: Config>() -> u32 {
        0
    }

    #[pallet::storage]
    #[pallet::getter(fn attestor_num)]
    pub type AttestorNum<T: Config> = StorageValue<_, u32, ValueQuery, DefaultAttestorNum<T>>;

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Attestor registered. \[attestor_id\]
        AttestorRegister(T::AccountId),
        /// Attestor moved. \[attestor_id\]
        AttestorRemove(T::AccountId),
        /// Attestor's url updated. \[attestor_id\]
        AttestorUpdate(T::AccountId),
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(u32, T::AccountId),
        ApplicationAttested(T::AccountId, T::AccountId),
        AttestorReport(T::AccountId, T::AccountId),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Use an invalid attestor id.
        InvalidAttestor,
        /// Attestor not whitelisted.
        AttestorNotWhitelisted,
        /// Attestor already registered.
        AlreadyRegistered,
        /// Invalid notification input.
        InvalidNotification,
        /// Attestor not attesting this geode.
        NotAttestingFor,
        /// Attestor has already attest the application before
        DuplicateAttestation,
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::attestor_heartbeat(message, signature_raw_bytes) => {
                    // validate inputs
                    if message.len() != 40 {
                        return InvalidTransaction::Call.into();
                    }

                    let mut attestor = [0u8; 32];
                    attestor.copy_from_slice(&message[0..32]);

                    let pubkey = Public::from_raw(attestor.clone());
                    let signature = Signature::from_raw(signature_raw_bytes.clone());
                    #[cfg(feature = "full_crypto")]
                    if !Sr25519Pair::verify(&signature, message, &pubkey) {
                        return InvalidTransaction::Call.into();
                    }

                    let acc = T::AccountId::decode(&mut &attestor[..]).unwrap_or_default();
                    if !<Attestors<T>>::contains_key(acc) {
                        return InvalidTransaction::Call.into();
                    }

                    ValidTransaction::with_tag_prefix("Automata/attestor/heartbeat")
                        .priority(UNSIGNED_TXS_PRIORITY)
                        .and_provides((
                            attestor,
                            <frame_system::Pallet<T>>::block_number()
                                .saturated_into::<BlockNumber>(),
                        ))
                        .longevity(3)
                        .propagate(true)
                        .build()
                }
                _ => InvalidTransaction::Call.into(),
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: T::BlockNumber) -> Weight {
            if let Ok(now) = TryInto::<BlockNumber>::try_into(block_number) {
                let mut expired_attestors = Vec::<T::AccountId>::new();
                for (key, notify) in <AttestorLastNotify<T>>::iter() {
                    if notify + T::HeartbeatTimeoutBlockNumber::get() < now {
                        expired_attestors.push(key);
                    }
                }
                for key in expired_attestors {
                    Self::attestor_remove(RawOrigin::Signed(key).into());
                }
            }
            0
        }
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn set_whitelist(
            origin: OriginFor<T>,
            account: T::AccountId,
            whitelisted: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <AttestorsWhiteList<T>>::insert(account, whitelisted);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn set_attestor_numbers(
            origin: OriginFor<T>,
            minimum: u16,
            expected: u16,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <MinimumAttestorNum<T>>::set(minimum);
            <ExpectedAttestorNum<T>>::set(expected);
            Ok(().into())
        }

        /// Register as an attestor.
        #[pallet::weight(0)]
        pub fn attestor_register(
            origin: OriginFor<T>,
            url: Vec<u8>,
            pubkey: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                !<Attestors<T>>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );
            ensure!(
                <AttestorsWhiteList<T>>::get(&who),
                <Error<T>>::AttestorNotWhitelisted
            );

            let attestor = AttestorOf::<T> {
                url,
                pubkey,
                applications: BTreeSet::new(),
            };
            <Attestors<T>>::insert(&who, attestor);

            let block_number =
                <frame_system::Pallet<T>>::block_number().saturated_into::<BlockNumber>();
            <AttestorLastNotify<T>>::insert(&who, block_number);

            <AttestorNum<T>>::put(<AttestorNum<T>>::get() + 1);

            Self::deposit_event(Event::AttestorRegister(who));
            Ok(().into())
        }

        /// Attestor heartbeat
        #[pallet::weight(0)]
        pub fn attestor_heartbeat(
            _origin: OriginFor<T>,
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> DispatchResultWithPostInfo {
            // validate inputs
            ensure!(message.len() == 40, Error::<T>::InvalidNotification);

            let mut attestor = [0u8; 32];
            attestor.copy_from_slice(&message[0..32]);

            let pubkey = Public::from_raw(attestor.clone());
            let signature = Signature::from_raw(signature_raw_bytes.clone());

            #[cfg(feature = "full_crypto")]
            ensure!(
                Sr25519Pair::verify(&signature, message, &pubkey),
                Error::<T>::InvalidNotification
            );

            let acc = T::AccountId::decode(&mut &attestor[..]).unwrap_or_default();
            ensure!(
                <Attestors<T>>::contains_key(&acc),
                Error::<T>::InvalidAttestor
            );

            let block_number =
                <frame_system::Pallet<T>>::block_number().saturated_into::<BlockNumber>();
            <AttestorLastNotify<T>>::insert(&acc, block_number);
            Ok(().into())
        }

        /// Update attestor's url
        #[pallet::weight(0)]
        pub fn attestor_update(origin: OriginFor<T>, url: Vec<u8>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                <Attestors<T>>::contains_key(&who),
                Error::<T>::InvalidAttestor
            );
            let mut attestor = <Attestors<T>>::get(&who);
            attestor.url = url;
            <Attestors<T>>::insert(&who, attestor);
            Self::deposit_event(Event::AttestorUpdate(who));
            Ok(().into())
        }

        /// Remove attestor
        #[pallet::weight(0)]
        pub fn attestor_remove(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                Attestors::<T>::contains_key(&who),
                Error::<T>::InvalidAttestor
            );

            let attestor_record = <Attestors<T>>::get(&who);
            let minimum_attestor_num = <MinimumAttestorNum<T>>::get() as usize;
            for application_id in attestor_record.applications.iter() {
                let mut attestors = AttestorsOfApplications::<T>::get(&application_id);
                attestors.remove(&who);
                if attestors.is_empty() {
                    AttestorsOfApplications::<T>::remove(&application_id);
                } else {
                    AttestorsOfApplications::<T>::insert(&application_id, &attestors);
                }
                if minimum_attestor_num > attestors.len() {
                    let _ =
                        T::ApplicationHandler::application_unhealthy(application_id.clone(), true);
                }
            }

            AttestorNum::<T>::put(AttestorNum::<T>::get().saturating_sub(1));
            AttestorLastNotify::<T>::remove(&who);
            Attestors::<T>::remove(&who);

            Self::deposit_event(Event::AttestorRemove(who));

            Ok(().into())
        }

        // Attestor report that an application is attested by it
        #[pallet::weight(0)]
        pub fn attestor_attest_application(
            origin: OriginFor<T>,
            application_id: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                Attestors::<T>::contains_key(&who),
                Error::<T>::InvalidAttestor
            );

            let mut attestor_record = Attestors::<T>::get(&who);
            ensure!(
                !attestor_record.applications.contains(&application_id),
                Error::<T>::DuplicateAttestation
            );
            attestor_record.applications.insert(application_id.clone());
            Attestors::<T>::insert(&who, attestor_record);

            let mut attestors = BTreeSet::<T::AccountId>::new();
            if AttestorsOfApplications::<T>::contains_key(&application_id) {
                attestors = AttestorsOfApplications::<T>::get(&application_id);
            }
            attestors.insert(who.clone());
            if attestors.len() as u16 == <MinimumAttestorNum<T>>::get() {
                T::ApplicationHandler::application_healthy(application_id.clone())?;
            }

            AttestorsOfApplications::<T>::insert(&application_id, attestors);

            Self::deposit_event(Event::ApplicationAttested(application_id, who));

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn attestor_report(
            origin: OriginFor<T>,
            application_id: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                Attestors::<T>::contains_key(&who),
                Error::<T>::InvalidAttestor
            );
            ensure!(
                Attestors::<T>::get(&who)
                    .applications
                    .contains(&application_id),
                Error::<T>::NotAttestingFor
            );

            let mut attestors = AttestorsOfApplications::<T>::get(&application_id);
            attestors.remove(&who);

            if (attestors.len() as u16) < <MinimumAttestorNum<T>>::get() {
                T::ApplicationHandler::application_unhealthy(application_id.clone(), false)?;
            }

            if attestors.is_empty() {
                AttestorsOfApplications::<T>::remove(&application_id);
            } else {
                AttestorsOfApplications::<T>::insert(&application_id, attestors);
            }

            let mut attestor = Attestors::<T>::get(&who);
            attestor.applications.remove(&application_id);
            Attestors::<T>::insert(&who, attestor);

            Self::deposit_event(Event::AttestorReport(who, application_id));

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn unsigned_attestor_heartbeat(
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> Result<(), ()> {
            let call = Call::attestor_heartbeat(message, signature_raw_bytes);
            SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
        }

        pub fn get_all_attestors() -> BTreeMap<T::AccountId, usize> {
            let mut result = BTreeMap::new();
            let iterator = <Attestors<T>>::iter().map(|(account_id, attestor)| {
                result.insert(account_id, attestor.applications.len());
            });
            result
        }

        /// Return attestors' url and pubkey list for rpc.
        pub fn attestor_list() -> Vec<(Vec<u8>, Vec<u8>, u32)> {
            let mut res = Vec::<(Vec<u8>, Vec<u8>, u32)>::new();
            <Attestors<T>>::iter()
                .map(|(_, attestor)| {
                    res.push((
                        attestor.url.clone(),
                        attestor.pubkey,
                        attestor.applications.len() as u32,
                    ));
                })
                .all(|_| true);
            res
        }

        /// Return list of attestors of a geode
        pub fn attestors_of_geode(geode: T::AccountId) -> Vec<(Vec<u8>, Vec<u8>)> {
            let mut res = Vec::new();
            let ids = <AttestorsOfApplications<T>>::get(&geode);
            ids.iter()
                .map(|id| {
                    let att = <Attestors<T>>::get(&id);
                    res.push((att.url, att.pubkey))
                })
                .all(|_| true);
            res
        }

        /// clean all the storage, USE WITH CARE!
        pub fn clean_storage() {
            // clean Attestors
            {
                let mut attestors = Vec::new();
                <Attestors<T>>::iter()
                    .map(|(key, _)| {
                        attestors.push(key);
                    })
                    .all(|_| true);
                for attestor in attestors.iter() {
                    <Attestors<T>>::remove(attestor);
                }
            }

            // clean GeodeAttestors
            {
                let mut attestors_of_applications = Vec::new();
                <AttestorsOfApplications<T>>::iter()
                    .map(|(key, _)| {
                        attestors_of_applications.push(key);
                    })
                    .all(|_| true);
                for geode_attestor in attestors_of_applications {
                    <AttestorsOfApplications<T>>::remove(geode_attestor);
                }
            }

            // clean AttestorLastNotify
            {
                let mut attestor_last_notifys = Vec::new();
                <AttestorLastNotify<T>>::iter()
                    .map(|(key, _)| {
                        attestor_last_notifys.push(key);
                    })
                    .all(|_| true);
                for attestor_last_notify in attestor_last_notifys.iter() {
                    <AttestorLastNotify<T>>::remove(attestor_last_notify);
                }
            }

            // reset AttestorNum
            <AttestorNum<T>>::put(0);
        }

        /// Return list apps an attestor is attesting
        pub fn attestor_attested_appids(attestor: T::AccountId) -> Vec<T::AccountId> {
            let mut res = Vec::new();
            let attestor = Attestors::<T>::get(&attestor);
            for app_id in &attestor.applications {
                res.push(app_id.clone());
            }
            res
        }
    }

    impl<T: Config> AttestorTrait for Pallet<T> {
        type AccountId = T::AccountId;
        fn is_abnormal_mode() -> bool {
            false
        }
        fn check_healthy(app_id: &Self::AccountId) -> bool {
            let attestor_list = <AttestorsOfApplications<T>>::get(app_id);
            attestor_list.len() >= <MinimumAttestorNum<T>>::get() as usize
        }
    }

    impl<T: Config> Get<BTreeMap<T::AccountId, usize>> for Pallet<T> {
        fn get() -> BTreeMap<T::AccountId, usize> {
            Self::get_all_attestors()
        }
    }
}
