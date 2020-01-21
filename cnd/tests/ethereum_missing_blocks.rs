pub mod ethereum_helper;

use chrono::NaiveDateTime;
use cnd::{
    btsieve::ethereum::{matching_transaction, TransactionPattern},
    ethereum::{Block, Transaction, TransactionAndReceipt, TransactionReceipt},
};
use ethereum_helper::EthereumConnectorMock;

#[tokio::test]
async fn find_transaction_in_missing_block_with_single_block_gap() {
    let transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/transaction.json"
    );
    let receipt: TransactionReceipt = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/receipt.json"
    );
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block4.json"
            ),
        ],
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block3_with_transaction.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block4.json"
            ),
        ],
        vec![(transaction.hash, receipt.clone())],
    );
    // Set start_of_swap to time of block 2, this allows us to go back to block 1.
    let block2: Block<Transaction> = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/block2.json"
    );
    let start_of_swap = NaiveDateTime::from_timestamp(block2.timestamp.as_u32() as i64, 0);

    let pattern = TransactionPattern {
        from_address: None,
        to_address: Some(transaction.to.unwrap()),
        is_contract_creation: None,
        transaction_data: None,
        transaction_data_length: None,
        events: None,
    };

    let expected_transaction_and_receipt = matching_transaction(connector, pattern, start_of_swap)
        .await
        .unwrap();

    assert_eq!(expected_transaction_and_receipt, TransactionAndReceipt {
        transaction,
        receipt
    });
}

#[tokio::test]
async fn find_transaction_in_missing_block_with_two_block_gap() {
    let transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/transaction.json"
    );
    let receipt: TransactionReceipt = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/receipt.json"
    );
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block5.json"
            ),
        ],
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block3_with_transaction.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block4.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block5.json"
            ),
        ],
        vec![(transaction.hash, receipt.clone())],
    );
    // Set start_of_swap to time of block 2, this allows us to go back to block 1.
    let block2: Block<Transaction> = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/block2.json"
    );
    let start_of_swap = NaiveDateTime::from_timestamp(block2.timestamp.as_u32() as i64, 0);

    let pattern = TransactionPattern {
        from_address: None,
        to_address: Some(transaction.to.unwrap()),
        is_contract_creation: None,
        transaction_data: None,
        transaction_data_length: None,
        events: None,
    };

    let expected_transaction_and_receipt = matching_transaction(connector, pattern, start_of_swap)
        .await
        .unwrap();

    assert_eq!(expected_transaction_and_receipt, TransactionAndReceipt {
        transaction,
        receipt
    });
}
