use anyhow::Result;
use pcap::Capture;
use crate::capture::decode;

#[test]
fn decap() -> Result<()> {
    let mut cap = Capture::from_file("pcaps/encap.pcap")?;
    let pkt  = cap.next()?;
    let flow = decode(None, pkt);
    assert!(flow.is_some());
    Ok(())
}
