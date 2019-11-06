use anyhow::Result;
use tokio::runtime::Runtime;
use kentik_api::*;

fn main() -> Result<()> {
    let rt = Runtime::new()?;

    let client = Client::new("test@example.com", "token", Some("localhost:8999"));
    // let client = Client::new("will@kentik.com", "5005335a7b859edb768b58925d1dfac1", Some("our1"));

    let device: Result<Device> = rt.block_on(async {
        let r = client.get_device_by_name("dev1").await?;
        println!("response {:?}", r);
        Ok(r)
    });

    // rt.block_on(async {
    //     let r = client.create_device(Device {
    //         name:        "foo".to_owned(),
    //         // kind:        "host-nprobe-dns-www".to_owned(),
    //         // subtype:     "kappa".to_owned(),
    //         bgp_type:    "none".to_owned(),
    //         cdn_attr:    "N".to_owned(),
    //         sample_rate: 1,
    //         plan_id:     Some(1),
    //         ..Default::default()
    //     }).await;
    //     println!("response {:?}", r);
    // });

    let device = device?;

    rt.spawn(async move {
        println!("sending flow");
        if let Err(e) = client.flow(&device, Vec::new()).await {
            println!("sending failed: {:?}", e);
        }
    });

    std::thread::sleep(std::time::Duration::from_secs(30));

    Ok(())
}
