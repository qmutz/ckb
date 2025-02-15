use crate::utils::wait_until;
use crate::{Net, Spec};
use log::info;
use std::{thread::sleep, time::Duration};

pub struct IBDProcess;

impl Spec for IBDProcess {
    crate::name!("ibd_process");

    crate::setup!(num_nodes: 7, connect_all: false);

    fn run(&self, net: Net) {
        info!("Running IBD process");

        let node0 = &net.nodes[0];
        let node1 = &net.nodes[1];
        let node2 = &net.nodes[2];
        let node3 = &net.nodes[3];
        let node4 = &net.nodes[4];
        let node5 = &net.nodes[5];
        let node6 = &net.nodes[6];

        node0.connect(node1);
        node0.connect(node2);
        node0.connect(node3);
        node0.connect(node4);
        // The node's outbound connection does not retain the peer which in the ibd state
        node0.generate_blocks(1);
        // will never connect
        node0.connect_uncheck(node5);
        node0.connect_uncheck(node6);

        sleep(Duration::from_secs(5));

        let rpc_client0 = node0.rpc_client();
        let is_connect_peer_num_eq_4 = wait_until(10, || {
            let peers = rpc_client0.get_peers();
            peers.len() == 4
        });

        if !is_connect_peer_num_eq_4 {
            panic!("refuse to connect fail");
        }

        // IBD only with outbound/whitelist node
        let rpc_client1 = node1.rpc_client();
        let rpc_client2 = node2.rpc_client();
        let rpc_client3 = node3.rpc_client();
        let rpc_client4 = node4.rpc_client();
        let rpc_client5 = node5.rpc_client();
        let rpc_client6 = node6.rpc_client();

        let is_nodes_ibd_sync = wait_until(10, || {
            let header1 = rpc_client1.get_tip_header();
            let header2 = rpc_client2.get_tip_header();
            let header3 = rpc_client3.get_tip_header();
            let header4 = rpc_client4.get_tip_header();
            let header5 = rpc_client5.get_tip_header();
            let header6 = rpc_client6.get_tip_header();

            header1.inner.number.value() == 0
                && header1 == header6
                && header1 == header5
                && header1 == header4
                && header1 == header3
                && header1 == header2
        });

        assert!(is_nodes_ibd_sync, "node 1-6 must not sync with node0");

        node5.connect(node0);
        node6.connect(node0);

        let is_node_sync = wait_until(10, || {
            let header5 = rpc_client5.get_tip_header();
            let header6 = rpc_client6.get_tip_header();
            header5 == header6 && header5.inner.number.value() == 1
        });

        assert!(is_node_sync, "node 5-6 must sync with node0");
    }
}

pub struct IBDProcessWithWhiteList;

impl Spec for IBDProcessWithWhiteList {
    crate::name!("ibd_process_with_whitelist");

    crate::setup!(num_nodes: 7, connect_all: false);

    fn run(&self, mut net: Net) {
        info!("Running IBD process with whitelist");

        {
            let node6_listen = format!(
                "/ip4/127.0.0.1/tcp/{}/p2p/{}",
                net.nodes[6].p2p_port(),
                net.nodes[6].node_id()
            );

            net.nodes[0].stop();

            // whitelist will be connected on outbound on node start
            net.nodes[0].start(
                Box::new(|_| ()),
                Box::new(move |config| {
                    config.network.whitelist_peers = vec![node6_listen.parse().unwrap()]
                }),
            );
        }

        let node0 = &net.nodes[0];
        let node1 = &net.nodes[1];
        let node2 = &net.nodes[2];
        let node3 = &net.nodes[3];
        let node4 = &net.nodes[4];
        let node5 = &net.nodes[5];
        let node6 = &net.nodes[6];

        node0.connect(node1);
        node0.connect(node2);
        node0.connect(node3);
        node0.connect(node4);

        // will never connect, protect node default is 4, see
        // https://github.com/nervosnetwork/ckb/blob/da8897dbc8382293bdf8fadea380a0b79c1efa92/sync/src/lib.rs#L57
        node0.connect_uncheck(node5);

        let rpc_client0 = node0.rpc_client();
        let is_connect_peer_num_eq_5 = wait_until(10, || {
            let peers = rpc_client0.get_peers();
            peers.len() == 5
        });

        if !is_connect_peer_num_eq_5 {
            panic!("refuse to connect fail");
        }

        node6.disconnect(node0);
        node6.generate_blocks(2);

        let generate_res = wait_until(10, || net.nodes[6].get_tip_block_number() == 2);

        if !generate_res {
            panic!("node6 can't generate blocks to 2");
        }

        node0.connect_uncheck(node6);

        // IBD only with outbound/whitelist node
        let rpc_client1 = node1.rpc_client();
        let rpc_client2 = node2.rpc_client();
        let rpc_client3 = node3.rpc_client();
        let rpc_client4 = node4.rpc_client();
        let rpc_client5 = node5.rpc_client();
        let rpc_client6 = node6.rpc_client();

        let is_nodes_ibd_sync = wait_until(10, || {
            let header0 = rpc_client0.get_tip_header();
            let header1 = rpc_client1.get_tip_header();
            let header2 = rpc_client2.get_tip_header();
            let header3 = rpc_client3.get_tip_header();
            let header4 = rpc_client4.get_tip_header();
            let header5 = rpc_client5.get_tip_header();
            let header6 = rpc_client6.get_tip_header();

            header1.inner.number.value() == 0
                && header1 == header5
                && header1 == header4
                && header1 == header3
                && header1 == header2
                && header6.inner.number.value() == 2
                && header0 == header6
        });

        assert!(
            is_nodes_ibd_sync,
            "node 1-5 must not sync with node0, node 6 must sync with node0"
        );
    }
}
