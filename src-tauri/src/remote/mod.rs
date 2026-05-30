pub mod store;
pub mod types;

pub use store::validate_profile;
pub use types::{
    RemoteAuthMethod, RemoteCapability, RemoteCommandError, RemoteCommandRequest,
    RemoteCommandResponse, RemoteHealth, RemoteHostProfile, RemotePlatform,
};
