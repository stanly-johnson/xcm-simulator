use frame_support::{parameter_types, traits::Get};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32};
use sp_std::cell::RefCell;

use polkadot_parachain::primitives::{Sibling, UpwardMessage};
use xcm::v0::{Junction, MultiLocation, NetworkId, OriginKind, SendXcm, Xcm};
use xcm_builder::{
	AccountId32Aliases, LocationInverter, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SovereignSignedViaLocation,
};
use xcm_executor::{traits::NativeAsset, Config, XcmExecutor};

use cumulus_primitives_core::{
	HrmpMessageHandler, HrmpMessageSender, InboundHrmpMessage, OutboundHrmpMessage, UpwardMessageSender,
};

pub mod para_a {
	use super::*;

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
	}

	pub type AccountId = AccountId32;

	impl frame_system::Config for Runtime {
		type Origin = Origin;
		type Call = Call;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = ::sp_runtime::traits::BlakeTwo256;
		type AccountId = AccountId;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = Event;
		type BlockHashCount = BlockHashCount;
		type BlockWeights = ();
		type BlockLength = ();
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type DbWeight = ();
		type BaseCallFilter = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
	}

	impl parachain_info::Config for Runtime {}

	parameter_types! {
		pub Network: NetworkId = NetworkId::Any;
		pub RelayChainOrigin: Origin = cumulus_pallet_xcm_handler::Origin::Relay.into();
		pub Ancestry: MultiLocation = MultiLocation::X1(Junction::Parachain {
			id: ParachainInfo::get().into(),
		});
	}

	pub type LocationConverter = (
		ParentIsDefault<AccountId>,
		SiblingParachainConvertsVia<Sibling, AccountId>,
		AccountId32Aliases<Network, AccountId>,
	);

	pub type LocalOriginConverter = (
		SovereignSignedViaLocation<LocationConverter, Origin>,
		RelayChainAsNative<RelayChainOrigin, Origin>,
		SiblingParachainAsNative<cumulus_pallet_xcm_handler::Origin, Origin>,
		SignedAccountId32AsNative<Network, Origin>,
	);

	pub struct XcmConfig;
	impl Config for XcmConfig {
		type Call = Call;
		type XcmSender = XcmHandler;
		type AssetTransactor = ();
		type OriginConverter = LocalOriginConverter;
		//TODO: might need to add other assets based on orml-tokens
		type IsReserve = NativeAsset;
		type IsTeleporter = ();
		type LocationInverter = LocationInverter<Ancestry>;
	}

	pub struct MockMessenger;
	impl UpwardMessageSender for MockMessenger {
		fn send_upward_message(msg: UpwardMessage) -> Result<(), ()> {
			Sim::send_ump_msg(ParachainInfo::parachain_id().into(), msg)
		}
	}

	impl HrmpMessageSender for MockMessenger {
		fn send_hrmp_message(msg: OutboundHrmpMessage) -> Result<(), ()> {
			let OutboundHrmpMessage { recipient, data } = msg;
			Sim::send_hrmp_msg(ParachainInfo::parachain_id().into(), recipient.into(), data)
		}
	}

	impl cumulus_pallet_xcm_handler::Config for Runtime {
		type Event = Event;
		type XcmExecutor = XcmExecutor<XcmConfig>;
		type UpwardMessageSender = MockMessenger;
		type HrmpMessageSender = MockMessenger;
		type SendXcmOrigin = EnsureRoot<AccountId>;
		type AccountIdConverter = LocationConverter;
	}

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
	type Block = frame_system::mocking::MockBlock<Runtime>;

	frame_support::construct_runtime!(
		pub enum Runtime where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
			ParachainInfo: parachain_info::{Pallet, Storage, Config},
			XcmHandler: cumulus_pallet_xcm_handler::{Pallet, Call, Event<T>, Origin},
		}
	);
}

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build<Runtime: frame_system::Config>(self, para_id: u32) -> sp_io::TestExternalities
	where
		<Runtime as frame_system::Config>::BlockNumber: From<u64>,
	{
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		parachain_info::GenesisConfig {
			parachain_id: para_id.into(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| frame_system::Pallet::<Runtime>::set_block_number(1.into()));
		ext
	}
}

pub fn ext<Runtime: frame_system::Config>(para_id: u32) -> sp_io::TestExternalities
where
	<Runtime as frame_system::Config>::BlockNumber: From<u64>,
{
	ExtBuilder::default().build::<Runtime>(para_id)
}

//TODO: pattern
thread_local! {
	pub static EXT_111: RefCell<TestExternalities> = RefCell::new(ext::<para_a::Runtime>(111));
	pub static EXT_222: RefCell<TestExternalities> = RefCell::new(ext::<para_a::Runtime>(222));
}

pub struct Sim;
impl Sim {
	pub fn reset_ext() {
		//TODO: pattern
		EXT_111.with(|v| *v.borrow_mut() = ext::<para_a::Runtime>(111));
		EXT_222.with(|v| *v.borrow_mut() = ext::<para_a::Runtime>(222));
	}

	pub fn execute_with<R>(para_id: u32, execute: impl FnOnce() -> R) -> R {
		match para_id {
			//TODO: pattern
			111 => EXT_111.with(|v| v.borrow_mut().execute_with(execute)),
			222 => EXT_222.with(|v| v.borrow_mut().execute_with(execute)),
			_ => unreachable!("ext has been set"),
		}
	}

	fn send_ump_msg(from: u32, msg: Vec<u8>) -> Result<(), ()> {
		println!("Sim ump sent: from {:?}, msg {:?}", from, msg);
		Ok(())
	}

	fn send_hrmp_msg(from: u32, to: u32, msg: Vec<u8>) -> Result<(), ()> {
		println!("Sim hrmp sent: from {:?}, to {:?}, msg {:?}", from, to, msg);
		match to {
			//TODO: pattern
			111 | 222 => {
				Self::execute_with(to, || {
					para_a::XcmHandler::handle_hrmp_message(from.into(), InboundHrmpMessage { sent_at: 10, data: msg });
				});
				Ok(())
			}
			_ => Err(()),
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    
	#[test]
	fn try_xcm() {
		Sim::reset_ext();

		Sim::execute_with(111, || {
			<para_a::XcmHandler as SendXcm>::send_xcm(
				(Junction::Parent, Junction::Parachain { id: 222 }).into(),
				Xcm::Transact {
					origin_type: OriginKind::Native,
					call: vec![1],
				},
			);
			println!("-------- 111 events");
			para_a::System::events()
				.iter()
				.for_each(|r| println!(">>> {:?}", r.event));
		});

		Sim::execute_with(222, || {
			println!("-------- 222 events");
			para_a::System::events()
				.iter()
				.for_each(|r| println!(">>> {:?}", r.event));
		});

		Sim::reset_ext();

		Sim::execute_with(111, || {
			println!("-------- 111 events");
			para_a::System::events()
				.iter()
				.for_each(|r| println!(">>> {:?}", r.event));
		});

		Sim::execute_with(222, || {
			println!("-------- 222 events");
			para_a::System::events()
				.iter()
				.for_each(|r| println!(">>> {:?}", r.event));
		});
	}
}