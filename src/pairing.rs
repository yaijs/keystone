use crate::config::HostFlavor;
use crate::protocol::PairingStatus;

#[derive(Debug, Clone)]
pub struct TrustRecord {
    pub host_flavor: HostFlavor,
    pub extension_id: String,
    pub extension_name: String,
    pub allowed_providers: Vec<String>,
}

#[derive(Debug, Default)]
pub struct PairingStore {
    record: Option<TrustRecord>,
}

impl PairingStore {
    pub fn from_record(record: Option<TrustRecord>) -> Self {
        Self { record }
    }

    pub fn current_status(&self, host_flavor: HostFlavor, extension_id: &str) -> PairingStatus {
        match &self.record {
            Some(record)
                if record.host_flavor == host_flavor && record.extension_id == extension_id =>
            {
                PairingStatus::Paired
            }
            _ => PairingStatus::Unpaired,
        }
    }

    pub fn get_record(&self, host_flavor: HostFlavor, extension_id: &str) -> Option<TrustRecord> {
        self.record
            .as_ref()
            .filter(|record| {
                record.host_flavor == host_flavor && record.extension_id == extension_id
            })
            .cloned()
    }

    pub fn current_record(&self) -> Option<TrustRecord> {
        self.record.clone()
    }

    pub fn pair_extension(
        &mut self,
        host_flavor: HostFlavor,
        extension_id: String,
        extension_name: String,
        requested_providers: Vec<String>,
    ) -> TrustRecord {
        let record = TrustRecord {
            host_flavor,
            extension_id,
            extension_name,
            allowed_providers: requested_providers,
        };
        self.record = Some(record.clone());
        record
    }
}
