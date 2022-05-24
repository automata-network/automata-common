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
    use automata_traits::AttestorAccounting;
    use frame_support::traits::{Currency, ReservableCurrency};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo, pallet_prelude::*, unsigned::ValidateUnsigned,
    };
    use frame_system::{
        offchain::{SendTransactionTypes, SubmitTransaction},
        pallet_prelude::*,
    };
    use primitives::{BlockNumber, attestor::ReportType};
    use automata_traits::attestor::ApplicationTrait;
    #[cfg(feature = "full_crypto")]
    use sp_core::crypto::Pair;
    #[cfg(feature = "full_crypto")]
    use sp_core::sr25519::Pair as Sr25519Pair;
    use sp_core::sr25519::{Public, Signature};
    use sp_runtime::{RuntimeDebug, SaturatedConversion, Percent};
    use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
    use sp_std::prelude::*;

    /// Attestor struct
    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
    pub struct Attestor<AccountId: Ord> {
        /// Attestor's url, geode will get it and communicate with attestor.
        pub url: Vec<u8>,
        /// Attestor's Secp256r1PublicKey
        pub pubkey: Vec<u8>,
        /// Geode being attested by this attestor
        pub geodes: BTreeSet<AccountId>,
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
        type AttestorAccounting: AttestorAccounting<AccountId = Self::AccountId>;
        type ApplicationHandler: ApplicationTrait;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // The pallet's runtime storage items.
    #[pallet::storage]
    #[pallet::getter(fn attestors)]
    pub type Attestors<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AttestorOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn geode_attestors)]
    pub type GeodeAttestors<T: Config> =
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
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Use an invalid attestor id.
        InvalidAttestor,
        /// Attestor already registered.
        AlreadyRegistered,
        /// Invalid notification input.
        InvalidNotification,
        /// Attestor not attesting this geode.
        NotAttestingFor,
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::attestor_notify_chain(message, signature_raw_bytes) => {
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

                    ValidTransaction::with_tag_prefix("Automata/attestor/notify")
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
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
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
            T::AttestorAccounting::attestor_staking(who.clone().into())?;

            let attestor = AttestorOf::<T> {
                url,
                pubkey,
                geodes: BTreeSet::new(),
            };
            <Attestors<T>>::insert(&who, attestor);

            let block_number =
                <frame_system::Pallet<T>>::block_number().saturated_into::<BlockNumber>();
            <AttestorLastNotify<T>>::insert(&who, block_number);

            <AttestorNum<T>>::put(<AttestorNum<T>>::get() + 1);

            Self::deposit_event(Event::AttestorRegister(who));
            Ok(().into())
        }

        /// Called by attestor to update its url.
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

        #[pallet::weight(0)]
        pub fn attestor_notify_chain(
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

        /// Report that somebody did a misconduct. The actual usage is being considered.
        #[pallet::weight(0)]
        pub fn attestor_report(
            origin: OriginFor<T>,
            geode_id: T::AccountId,
            report_type: u8,
            _proof: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            // check attestor existance and whether attested
            ensure!(
                <Attestors<T>>::contains_key(&who),
                Error::<T>::InvalidAttestor
            );
            ensure!(
                <Attestors<T>>::get(&who)
                    .geodes
                    .contains(&geode_id),
                Error::<T>::NotAttestingFor
            );
            // check have report
            match ReportType::try_from(report_type) {
                Ok(t) => {
                    match t {
                        ReportType::Challenge => {
                            let geode = pallet_geode::Geodes::<T>::get(&geode_id);
                            if geode.state == pallet_geode::GeodeState::Registered {
                                // just exit attesting for it
                                let mut attestors =
                                    <GeodeAttestors<T>>::get(&geode_id);
                                attestors.remove(&who);

                                if attestors.is_empty() {
                                    <GeodeAttestors<T>>::remove(&geode_id);
                                } else {
                                    <GeodeAttestors<T>>::insert(
                                        &geode_id, &attestors,
                                    );
                                }

                                Self::deposit_event(Event::ReportBlame(who, geode_id));
                                return Ok(().into());
                            }
                            ensure!(
                                geode.state == pallet_geode::GeodeState::Attested
                                    || geode.state == pallet_geode::GeodeState::Instantiated
                                    || geode.state == pallet_geode::GeodeState::Degraded,
                                pallet_geode::Error::<T>::InvalidGeodeState
                            );
                        }
                        ReportType::Service => {
                            let geode = pallet_geode::Geodes::<T>::get(&geode_id);
                            ensure!(
                                geode.state == pallet_geode::GeodeState::Instantiated
                                    || geode.state == pallet_geode::GeodeState::Degraded,
                                pallet_geode::Error::<T>::InvalidGeodeState
                            );
                            let service_use =
                                pallet_service::Services::<T>::get(geode.order.unwrap().0);
                            ensure!(
                                service_use.geodes.contains(&geode_id),
                                pallet_service::Error::<T>::InvalidServiceState
                            );
                        }
                        _ => {
                            return Err(Error::<T>::InvalidReportType.into());
                        }
                    }
                }
                Err(_) => {
                    return Err(Error::<T>::InvalidReportType.into());
                }
            };

            let mut attestors = <GeodeAttestors<T>>::get(&geode_id);
            attestors.remove(&who);

            if attestors.is_empty() {
                <GeodeAttestors<T>>::remove(&geode_id);
            } else {
                <GeodeAttestors<T>>::insert(&geode_id, &attestors);
            }

            let mut attestorOf = <Attestors<T>>::get(&who);
            attestorOf.geodes.remove(&geode_id);
            <Attestors<T>>::insert(&who, attestorOf);

            // ApplicationHandler::unhealthy_application(&geode_id, ReportType::try_from(report_type));
            // 如果每次report都移除attestor，那会导致不同原因的report最终混合导致geode状态改变


            let key = (geode_id.clone(), report_type);
            let mut report = ReportOf::<T>::default();
            if <Reports<T>>::contains_key(&key) {
                report = <Reports<T>>::get(&key);
                report.attestors.insert(who.clone());
            } else {
                report.attestors.insert(who.clone());
                let block_number =
                    <frame_system::Module<T>>::block_number().saturated_into::<BlockNumber>();
                report.start = block_number;
            }

            // check current amount of misconduct satisfying the approval ratio
            if Percent::from_rational_approximation(
                report.attestors.len(),
                <GeodeAttestors<T>>::get(&geode_id).len(),
            ) >= T::ReportApprovalRatio::get()
            {
                // slash the geode
                Self::slash_geode(&key.0);
                <Reports<T>>::remove(&key);
                Self::deposit_event(Event::SlashGeode(key.0.clone()));
            } else {
                // update report storage
                <Reports<T>>::insert(&key, report);
            }

            Self::deposit_event(Event::ReportBlame(who, key.0));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn unsigned_attestor_notify_chain(
            message: Vec<u8>,
            signature_raw_bytes: [u8; 64],
        ) -> Result<(), ()> {
            let call = Call::attestor_notify_chain(message, signature_raw_bytes);
            SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
        }

        pub fn get_all_attestors() -> BTreeMap<T::AccountId, usize> {
            let mut result = BTreeMap::new();
            let iterator = <Attestors<T>>::iter().map(|(accountId, attestor)| {
                result.insert(accountId, attestor.geodes.len());
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
                        attestor.geodes.len() as u32,
                    ));
                })
                .all(|_| true);
            res
        }

        /// Return list of attestors of a geode
        pub fn attestors_of_geode(geode: T::AccountId) -> Vec<(Vec<u8>, Vec<u8>)> {
            let mut res = Vec::new();
            let ids = <GeodeAttestors<T>>::get(&geode);
            ids.iter()
                .map(|id| {
                    let att = <Attestors<T>>::get(&id);
                    res.push((att.url, att.pubkey))
                })
                .all(|_| true);
            res
        }

        /// remove attestor, return degraded geodes
        pub fn attestor_remove(attestor: T::AccountId) -> Vec<T::AccountId> {
            let attestor_record = <Attestors<T>>::get(&attestor);

            let mut ret = Vec::new();

            for geode in attestor_record.geodes.into_iter() {
                ret.push(geode)
            }

            // change storage
            <AttestorNum<T>>::put(<AttestorNum<T>>::get() - 1);
            <AttestorLastNotify<T>>::remove(&attestor);
            <Attestors<T>>::remove(&attestor);

            // deposit event
            Self::deposit_event(Event::AttestorRemove(attestor));

            ret
        }

        /// detach geode from attestors
        pub fn detach_geode_from_attestors(geode: &T::AccountId) {
            // clean record on attestors
            if GeodeAttestors::<T>::contains_key(&geode) {
                for id in GeodeAttestors::<T>::get(&geode) {
                    let mut attestor = Attestors::<T>::get(&id);
                    attestor.geodes.remove(&geode);
                    Attestors::<T>::insert(&id, attestor);
                }
                GeodeAttestors::<T>::remove(&geode);
            }
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
                let mut geode_attestors = Vec::new();
                <GeodeAttestors<T>>::iter()
                    .map(|(key, _)| {
                        geode_attestors.push(key);
                    })
                    .all(|_| true);
                for geode_attestor in geode_attestors.iter() {
                    <GeodeAttestors<T>>::remove(geode_attestor);
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
    }

    impl<T: Config> Get<BTreeMap<T::AccountId, usize>> for Pallet<T> {
        fn get() -> BTreeMap<T::AccountId, usize> {
            Self::get_all_attestors()
        }
    }
}
