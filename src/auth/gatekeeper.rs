use uuid::Uuid;
use windows::core::GUID;

pub struct GateKeeperProvider;

impl GateKeeperProvider {
    pub fn generate_id() -> Result<GUID, String> {
        let uuid = Uuid::new_v4();
        let (d1, d2, d3, d4) = uuid.as_fields();
        Ok(GUID::from_values(d1, d2, d3, *d4))
    }
}
