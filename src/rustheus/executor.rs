use chain::{BlockHeader, Block, Transaction, TransactionInput, TransactionOutput};
use crypto::DHash256;
use std::sync::mpsc::{self, Sender, Receiver};
use mempool::{MempoolRef};
use std::time::{SystemTime, UNIX_EPOCH};
use executor_tasks::Task;
use message::types::{Block as BlockMessage};
use message_wrapper::MessageWrapper;
use db::SharedStore;
use keys::Address;
use script::Builder;

pub struct Executor
{
    task_receiver: Receiver<Task>,
    message_wrapper: MessageWrapper,
    mempool: MempoolRef,
    storage: SharedStore
}

impl Executor
{
    pub fn new(mempool: MempoolRef, storage: SharedStore, message_wrapper: MessageWrapper) -> (Self, Sender<Task>)
    {
        let (task_sender, task_receiver) = mpsc::channel();
        let executor = Executor
        {
            task_receiver,
            message_wrapper,
            mempool,
            storage,
        };
        (executor, task_sender)
    }

    pub fn run(&mut self)
    {
        loop
        {
            if let Ok(task) = self.task_receiver.recv()
            {
                info!("task received, it is {:?}", task);
                match task
                {
                    Task::SignBlock(coinbase_recipient) => self.sign_block(coinbase_recipient),
                }
            }
            else
            {
                break;
            }
        } 
    }

    fn sign_block(&mut self, coinbase_recipient: Address)
    {
        let current_time = SystemTime::now();
        let time_since_the_epoch = current_time.duration_since(UNIX_EPOCH).expect("Time went backwards");

        let header = BlockHeader {
            version: 1,
            previous_header_hash: self.storage.best_block().hash,
            merkle_root_hash: DHash256::default().finish(),
            time: time_since_the_epoch.as_secs() as u32,
            bits: 5.into(),
            nonce: 6,
        };
        let mut mempool = self.mempool.write().unwrap();
        let mut transactions = vec![self.create_coinbase(coinbase_recipient)];
        transactions.extend(mempool.drain_as_vec());
        let mut block = Block::new(header, transactions);
        
        //recalculate merkle root
        block.block_header.merkle_root_hash = block.witness_merkle_root();

        let block_message = BlockMessage { block };
        self.message_wrapper.wrap(&block_message);
    }

    fn create_coinbase(&self, recipient: Address) -> Transaction
    {
        use chain::bytes::Bytes;
        Transaction {
            version: 0,
            inputs: vec![TransactionInput::coinbase(Bytes::default())],
            outputs: vec![TransactionOutput {
                value: 50,
                script_pubkey: Builder::build_p2pkh(&recipient.hash).to_bytes()
            }],
            lock_time: self.storage.best_block().number + 1, //use lock_time as uniqueness provider for coinbase transaction
        }
    }
}