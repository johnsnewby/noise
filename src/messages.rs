use byteorder::*;
use crate::rlp_val::*;
use rlp::{Rlp, RlpStream};

type RlpError = Box<std::error::Error>;

const MSG_FRAGMENT: u16 = 0;
const MSG_P2PRESPONSE: u16 = 100;
const MSG_PING: u16 = 1;
const MSG_GETHEADERBYHASH: u16 = 3;
const MSG_GETHEADERBYHEIGHT: u16 = 15;
const MSG_HEADER: u16 = 4;
const MSG_GETNSUCCESSORS: u16 = 5;
const MSG_HEADERHASHES: u16 = 6;
const MSG_GETBLOCKTXS: u16 = 7;
const MSG_GETGENERATION: u16 = 8;
const MSG_TXS: u16 = 9;
const MSG_BLOCKTXS: u16 = 13;
const MSG_KEYBLOCK: u16 = 10;
const MSG_MICROBLOCK: u16 = 11;
const MSG_GENERATION: u16 = 12;
const MSG_TXPOOLSYNCINIT: u16 = 20;
const MSG_TXPOOLSYNCUNFOLD: u16 = 21;
const MSG_TXPOOLSYNCGET: u16 = 22;
const MSG_TXPOOLSYNCFINISH: u16 = 23;
const MSG_CLOSE: u16 = 127;


fn display_message(msg_data: &Rlp) -> Result<(), RlpError> {
    println!("Starting message with {} elements:", msg_data.item_count()?);
    let mut i = msg_data.iter();
    loop {
        let ele;
        match i.next() {
            Some(x) => ele = x,
            None => break,
        };
        match ele.prototype().unwrap() {
            rlp::Prototype::Data(size) => println!("Data, size is {} content is {:?}",
                                                   size, ele.data().unwrap()),
            rlp::Prototype::List(count) => println!("List, length is {}", count),
            _ => println!("Something else"),
        };
    }
    println!("End message");
    Ok(())
}

pub fn handle_message(msg_type: u16, msg_data: &Rlp) -> Result<(), RlpError> {
    display_message(&msg_data)?;
    match msg_type {
        MSG_P2PRESPONSE => handle_p2p_response(&msg_data).unwrap(),
        MSG_TXPOOLSYNCINIT => handle_tx_pool_sync_init(&msg_data).unwrap(),
        MSG_TXS => handle_txs(&msg_data).unwrap(),
        MSG_KEYBLOCK => handle_key_blocks(&msg_data).unwrap(),
        MSG_MICROBLOCK => handle_micro_block(&msg_data).unwrap(),
        _ => (),
    }
    Ok(())
}

/*
Message is RLP encoded, fields:

Result :: bool - true means ok, false means error.
Type :: int - the type of the response
Reason :: byte_array - Human readable (UTF8) reason (only set if Result is false)*
Object :: byte_array - an object of type Type if Result is true.
*/
fn handle_p2p_response(msg_data: &Rlp) -> Result<(), RlpError> {
    let version: u8 = msg_data.val_at(0)?;
    let result: u8 = msg_data.val_at(1)?;
    let _type: u8 = msg_data.val_at(2)?;
    let reason: Vec<u8> = msg_data.val_at(3)?;
    let object: Vec<u8> = msg_data.val_at(4)?;
    println!(
        "p2p_response: version: {} result: {} type: {}, reason {:?} object: {:?}",
        version, result, _type, reason, object
    );
    let r = rlp::Rlp::new(&object);
    display_message(&r)?;
    Ok(())
}

/*
Message has no body.
*/
fn handle_tx_pool_sync_init(_msg_data: &Rlp) -> Result<(), RlpError> {
    Ok(())
}

/*
Message is RLP encoded, fields:

MicroBlock :: byte_array - Serialized micro block
Light :: bool - flag if micro block is light or normal

A normal micro block is serialized.  A light micro block is serialized
using aec_peer_connection:serialize_light_micro_block/1 - in effect
replacing the list of serialized signed transactions with a list of
transaction hashes.`

*/
fn handle_micro_block(msg_data: &Rlp) -> Result<(), RlpError> {
    let _version: u8 = msg_data.val_at(0)?;
    let _data = msg_data.at(1)?.data()?;
    let payload = &rlp::Rlp::new(&_data);
    display_message(&payload)?;
    let _light: u8 = msg_data.val_at(2)?;
    println!("Payload length is {}, _light is {}", payload.size(), _light);
    let mb = MicroBlockHeader::new_from_byte_array(&payload.at(2)?.data()?)?;
    let txs = payload.at(3)?;
    if ! _light == 0 {
        handle_txs(&txs)?;
    } else {
        let _r = crate::rlp_val::RlpVal::from_rlp(&txs)?;
        for i in 0 .. txs.item_count()? {
            println!("{}", crate::rlp_val::transaction_hash(&Vec::<u8>::convert(&_r[i])));
            let v = Vec::<u8>::convert(&_r[i]);
            match AeIdentifier::from_bytes(255, &v) {
                Some(x) => println!("{}", x),
                None => continue,
            };
        }
        display_message(&txs)?;
    }
    println!("{}", mb.to_string()?);
    Ok(())
}


#[test]
fn test_handle_micro_block() {
    let msg_data = include!("../data/micro-block.rs");
    display_message(&msg_data).unwrap();
    handle_micro_block(&msg_data).unwrap();
    println!("Done");
}


/*
 *
Message is RLP encoded, fields:

KeyBlock :: byte_array - Serialized key block
The key block is serialized.
*/
fn handle_key_blocks(msg_data: &Rlp) -> Result<(), RlpError> {
    assert!(msg_data.item_count()? % 2 == 0); // each KB is 2 nessages
    for i in 0..(msg_data.item_count()? / 2) {
        assert!(msg_data.val_at::<u16>(2 * i)? == 1);
        let data = msg_data.at(2 * i + 1)?.data()?;
        handle_key_block(&data)?
    }
    Ok(())
}

fn handle_key_block(binary: &[u8]) -> Result<(), RlpError> {
    let kb = KeyBlock::new_from_byte_array(binary)?;
    println!("height: {}", kb.height);
    println!("{}", kb.to_string()?);
    Ok(())
}

#[test]
fn test_handle_keyblocks() {
    let msg_data = include!("../data/key-block.rs");
    handle_key_blocks(&msg_data).unwrap();
}
/*

Message is RLP encoded, fields:

Txs:: [byte_array]
A signed transaction is serialized as a tagged and versioned signed transaction.
*/
pub fn handle_txs(msg_data: &Rlp) -> Result<(), RlpError> {
    println!("handle_txs, input is {:?}", msg_data);
    let version: u8 = msg_data.at(0).unwrap().data().unwrap()[0];
    assert!(version == 1);
    let tmp = msg_data.at(1).unwrap(); // temp variable so it doesn't go out of scope
    let mut iter = tmp.iter();
    for x in iter {
        let signed_tx = rlp::Rlp::new(x.data()?);
        let tx = RlpVal::from_rlp(&rlp::Rlp::new(signed_tx.at(3)?.data()?))?;
        let tag: u32 = u32::convert(&tx[0]);
        println!("{}", crate::jsonifier::process_tx(tag, &tx));
    }
    Ok(())
}

#[test]
fn test_handle_txs() {
    let txs = include!("../data/transactions.rs");
    for tx in txs {
        display_message(&tx).unwrap();
        handle_txs(&tx).unwrap();
    }
    /*
    let tmp = txs.at(1).unwrap();
    let mut iter = tmp.iter();
    let mut tx: rlp::Rlp;
    loop {
        let tx = match iter.next() {
            Some(x) => x,
            None => break,
        };
        let payload = rlp::Rlp::new(tx.data().unwrap());
        let rlp_val = RlpVal::from_rlp(&payload).unwrap();
        println!("rlp_val: {:?}", rlp_val);
        display_message(&payload);
        let unknown = rlp::Rlp::new(payload.at(3).unwrap().data().unwrap());
        let tx_ = RlpVal::from_rlp(&unknown).unwrap();
        println!("tx0 {:?}", tx_[0]);
        println!("rlp_val: {:?}", tx_);
        let _u: u32 = u32::convert(&tx_[0]);
        println!("tag: {}", _u);
        let ident: AeIdentifier = AeIdentifier::convert(&tx_[2]);
        println!("aeid: {}", ident);
        display_message(&unknown).unwrap();
        let unknown2 = unknown.at(8).unwrap().data().unwrap();
        println!("Payload is {}", String::convert(&tx_[8]));
        let v = crate::jsonifier::spend_tx(&tx_);
        println!("json is {}", v);
        println!("signed tx is {}", crate::jsonifier::signed_tx(&rlp_val).unwrap()); // TODO
    }
     */
}

pub fn bigend_u16(num: u16) -> Result<Vec<u8>, RlpError> {
    let mut v = vec![];
    v.write_u16::<BigEndian>(num)?;
    Ok(v)
}

/*
 * æternity expects RLP w/ some changes from the Parity
 */
pub fn mangle_rlp(data: &[u8]) -> Vec<u8> {
    data.iter()
        .map(|x| if *x == 128 { 0 } else { *x })
        .collect()
}

pub struct MicroBlockHeader {
    version: u32,
    tags: [u8; 4],
    height: u64,
    prev_hash: [u8; 32],
    prev_key_hash: [u8; 32],
    state_hash: [u8; 32],
    txs_hash: [u8; 32],
    time: u64,
    has_fraud: bool,
    fraud_hash: Option<[u8; 32]>,
    signature: [u8; 64],
}

/*
Fieldname	Size (bytes)
version	32 bits
micro_tag	1 bit
has_fraud	1 bit
unused_flags	30 bits (all set to 0)
height	8
prev_hash	32
prev_key_hash	32
state_hash	32
txs_hash	32
time	8
fraud_hash	0 or 32
signature	64
*/
impl MicroBlockHeader {
    fn new_from_byte_array(bytes: &[u8]) -> Result<MicroBlockHeader, RlpError> {
        println!("new mb from bytes: {:?}", bytes);

        let flags = array_ref![bytes, 4, 1][0];
        let _micro = flags & 0b1000_0000u8;
        let has_fraud = flags & 0b0100_0000u8 != 0;

        Ok(MicroBlockHeader {
            version: <&[u8]>::read_u32::<BigEndian>(&mut (&bytes[0..4]).clone())?,
            tags: array_ref![bytes, 4, 4].clone(),
            height: <&[u8]>::read_u64::<BigEndian>(&mut (&bytes[8..16]).clone())?,
            prev_hash: array_ref![bytes, 16, 32].clone(),
            prev_key_hash: array_ref![bytes, 48, 32].clone(),
            state_hash: array_ref![bytes, 80, 32].clone(),
            txs_hash: array_ref![bytes, 112, 32].clone(),
            time: <&[u8]>::read_u64::<BigEndian>(&mut (&bytes[144..152]).clone())?,
            has_fraud,
            fraud_hash: if has_fraud {
                Some(array_ref![bytes, 152, 32].clone())
            } else {
                None
            },
            signature: if has_fraud {
                array_ref![bytes, 184, 64].clone()
            } else {
                array_ref![bytes, 152, 64].clone()
            },
        })
    }

    pub fn to_string(&self) -> Result<String, RlpError> {
        Ok(format!(
            "version: {} flags: {:?} height: {} prev_hash: {:?} prev_key_hash: {:?} state_hash: {:?} \
             txs_hash: {:?} time: {} has_fraud: {} fraud_hash {:?}",
            self.version,
            self.tags,
            self.height,
            self.prev_hash,
            self.prev_key_hash,
            self.state_hash,
            self.txs_hash,
            self.time,
            self.has_fraud,
            self.fraud_hash,
        ))
    }
}

pub struct KeyBlock {
    version: u32,
    key_unused: u32,
    height: u64,
    prev_hash: [u8; 32],
    prev_key_hash: [u8; 32],
    state_hash: [u8; 32],
    miner: [u8; 32],
    beneficiary: [u8; 32],
    target: u32,
    pow: [u8; 168],
    nonce: u64,
    time: u64,
}

/*
Fieldname	Size (bytes)
version	32 bits
key_tag	1 bit
unused_flags	31 bits (all set to 0)
height	8
prev_hash	32
prev_key_hash	32
state_hash	32
miner	32
beneficiary	32
target	4
pow	168
nonce	8
time	8
*/
impl KeyBlock {
    fn new_from_byte_array(bytes: &[u8]) -> Result<KeyBlock, RlpError> {
        println!("bytes: {:?} length {}", bytes, bytes.len());
        let bytes = bytes.clone();
        Ok(KeyBlock {
            version: <&[u8]>::read_u32::<BigEndian>(&mut (&bytes[0..4]).clone())?,
            key_unused: <&[u8]>::read_u32::<BigEndian>(&mut (&bytes[4..8]).clone())?,
            height: <&[u8]>::read_u64::<BigEndian>(&mut (&bytes[8..16]).clone())?,
            prev_hash: array_ref![bytes, 16, 32].clone(),
            prev_key_hash: array_ref![bytes, 48, 32].clone(),
            state_hash: array_ref![bytes, 80, 32].clone(),
            miner: array_ref![bytes, 112, 32].clone(),
            beneficiary: array_ref![bytes, 144, 32].clone(),
            target: <&[u8]>::read_u32::<BigEndian>(&mut (&bytes[176..180]).clone())?,
            pow: array_ref![bytes, 180, 168].clone(),
            nonce: <&[u8]>::read_u64::<BigEndian>(&mut (&bytes[348..356]).clone())?,
            time: <&[u8]>::read_u64::<BigEndian>(&mut (&bytes[356..364]).clone())?,
        })
    }

    pub fn to_string(&self) -> Result<String, RlpError> {
        Ok(format!(
            "version: {} flags: {} height: {} prev_hash: {:?} prev_key_hash: {:?} state_hash: {:?} \
             miner: {:?} beneficiary: {:?} target: {} pow: {:?} nonce: {} time: {}",
            self.version,
            self.key_unused,
            self.height,
            self.prev_hash,
            self.prev_key_hash,
            self.state_hash,
            self.miner,
            self.beneficiary,
            self.target,
            self.pow.to_vec(),
            self.nonce,
            self.time
        ))
    }
}

#[derive(Debug, Serialize)]
pub struct Ping {
    version: u16,
    port: u16,
    share: u16,
    genesis_hash: Vec<u8>,
    difficulty: u64,
    top_hash: Vec<u8>,
    sync_allowed: u16,
    peers: Vec<u8>,
}

impl Ping {
    pub fn new(
        port: u16,
        share: u16,
        genesis_hash: Vec<u8>,
        difficulty: u64,
        top_hash: Vec<u8>,
        sync_allowed: bool,
        peers: Vec<u8>,
    ) -> Ping {
        Ping {
            version: 1,
            port,
            share,
            genesis_hash,
            difficulty,
            top_hash,
            sync_allowed: if sync_allowed { 1 } else { 0 },
            peers,
        }
    }

    pub fn rlp(&self) -> Result<Vec<u8>, Box<std::error::Error>> {
        let mut stream = RlpStream::new();
        let _peers: Vec<u8> = vec![];
        stream.begin_list(8).
            append(&1u16). // version
            append(&self.port).
            append(&self.share).
            append(&self.genesis_hash).
            append(&self.difficulty).
            append(&self.top_hash).
            append(&self.sync_allowed).
            begin_list(0);
        let v: Vec<u8> = stream.out();
        let mut v = mangle_rlp(&v);
        let version = bigend_u16(1)?;
        v.insert(0, version[0]); // message type
        v.insert(1, version[1]);
        println!("{:?}", v);
        Ok(v)
    }
}
