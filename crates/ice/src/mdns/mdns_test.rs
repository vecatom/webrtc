use super::*;
use crate::agent::{agent_config::*, agent_vnet_test::*, *};
use crate::candidate::*;
use crate::errors::*;
use crate::network_type::*;

use regex::Regex;
use tokio::sync::mpsc;

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_multicast_dns_only_connection() -> Result<(), Error> {
    let cfg0 = AgentConfig {
        network_types: vec![NetworkType::Udp4],
        candidate_types: vec![CandidateType::Host],
        multicast_dns_mode: MulticastDnsMode::QueryAndGather,
        ..Default::default()
    };

    let a_agent = Arc::new(Agent::new(cfg0).await?);
    let (a_notifier, mut a_connected) = on_connected();
    a_agent.on_connection_state_change(a_notifier).await;

    let cfg1 = AgentConfig {
        network_types: vec![NetworkType::Udp4],
        candidate_types: vec![CandidateType::Host],
        multicast_dns_mode: MulticastDnsMode::QueryAndGather,
        ..Default::default()
    };

    let b_agent = Arc::new(Agent::new(cfg1).await?);
    let (b_notifier, mut b_connected) = on_connected();
    b_agent.on_connection_state_change(b_notifier).await;

    connect_with_vnet(&a_agent, &b_agent).await?;
    let _ = a_connected.recv().await;
    let _ = b_connected.recv().await;

    a_agent.close().await?;
    b_agent.close().await?;

    Ok(())
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_multicast_dns_mixed_connection() -> Result<(), Error> {
    let cfg0 = AgentConfig {
        network_types: vec![NetworkType::Udp4],
        candidate_types: vec![CandidateType::Host],
        multicast_dns_mode: MulticastDnsMode::QueryAndGather,
        ..Default::default()
    };

    let a_agent = Arc::new(Agent::new(cfg0).await?);
    let (a_notifier, mut a_connected) = on_connected();
    a_agent.on_connection_state_change(a_notifier).await;

    let cfg1 = AgentConfig {
        network_types: vec![NetworkType::Udp4],
        candidate_types: vec![CandidateType::Host],
        multicast_dns_mode: MulticastDnsMode::QueryOnly,
        ..Default::default()
    };

    let b_agent = Arc::new(Agent::new(cfg1).await?);
    let (b_notifier, mut b_connected) = on_connected();
    b_agent.on_connection_state_change(b_notifier).await;

    connect_with_vnet(&a_agent, &b_agent).await?;
    let _ = a_connected.recv().await;
    let _ = b_connected.recv().await;

    a_agent.close().await?;
    b_agent.close().await?;

    Ok(())
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_multicast_dns_static_host_name() -> Result<(), Error> {
    let cfg0 = AgentConfig {
        network_types: vec![NetworkType::Udp4],
        candidate_types: vec![CandidateType::Host],
        multicast_dns_mode: MulticastDnsMode::QueryAndGather,
        multicast_dns_host_name: "invalidHostName".to_owned(),
        ..Default::default()
    };
    if let Err(err) = Agent::new(cfg0).await {
        assert_eq!(err, *ERR_INVALID_MULTICAST_DNSHOST_NAME);
    } else {
        assert!(false, "expected error, but got ok");
    }

    let cfg1 = AgentConfig {
        network_types: vec![NetworkType::Udp4],
        candidate_types: vec![CandidateType::Host],
        multicast_dns_mode: MulticastDnsMode::QueryAndGather,
        multicast_dns_host_name: "validName.local".to_owned(),
        ..Default::default()
    };

    let a = Agent::new(cfg1).await?;

    let (done_tx, mut done_rx) = mpsc::channel::<()>(1);
    let mut done_tx = Some(done_tx);
    a.on_candidate(Box::new(
        move |c: Option<Arc<dyn Candidate + Send + Sync>>| {
            if c.is_none() {
                done_tx.take();
            }
        },
    ))
    .await;

    a.gather_candidates().await?;

    log::debug!("wait for gathering is done...");
    let _ = done_rx.recv().await;
    log::debug!("gathering is done");

    Ok(())
}

#[test]
fn test_generate_multicast_dnsname() -> Result<(), Error> {
    let name = generate_multicast_dns_name();

    let re = Regex::new(
        r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}.local+$",
    );

    if let Ok(re) = re {
        assert!(
            re.is_match(&name),
            "mDNS name must be UUID v4 + \".local\" suffix, got {}",
            name
        );
    } else {
        assert!(false, "expected ok, but got err");
    }

    Ok(())
}
