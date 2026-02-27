//! MCP tool definitions with their JSON schemas.

use serde_json::json;

use super::types::McpTool;

/// Returns the list of all available MCP tools with their schemas.
pub fn get_tools() -> Vec<McpTool> {
    vec![
        // Node Management
        McpTool {
            name: "start_node".to_string(),
            description: "Start the Hathor fullnode. This must be running before mining or wallet operations.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "stop_node".to_string(),
            description: "Stop the Hathor fullnode and all related services (miner, wallet headless).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_node_status".to_string(),
            description: "Get the current status of the Hathor fullnode including block height and network info.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Miner Management
        McpTool {
            name: "start_miner".to_string(),
            description: "Start the CPU miner. The node must be running first.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "Mining reward address (uses node's wallet address if not provided)"
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "stop_miner".to_string(),
            description: "Stop the CPU miner.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_miner_status".to_string(),
            description: "Get the current status of the CPU miner.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Wallet Service
        McpTool {
            name: "start_wallet_service".to_string(),
            description: "Start the wallet-headless service for multi-wallet support. Node must be running first.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "stop_wallet_service".to_string(),
            description: "Stop the wallet-headless service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_wallet_service_status".to_string(),
            description: "Get the status of the wallet-headless service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Tx Mining Service
        McpTool {
            name: "start_tx_mining".to_string(),
            description: "Start the tx-mining-service. Required for wallet-headless transactions (send, publish blueprint, nano contracts). Node must be running first.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "stop_tx_mining".to_string(),
            description: "Stop the tx-mining-service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_tx_mining_status".to_string(),
            description: "Get the status of the tx-mining-service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Wallet Operations
        McpTool {
            name: "generate_seed".to_string(),
            description: "Generate a new 24-word BIP39 seed phrase for wallet creation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "create_wallet".to_string(),
            description: "Create a new wallet. Generates a seed if not provided.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "Unique identifier for the wallet"
                    },
                    "seed": {
                        "type": "string",
                        "description": "24-word BIP39 seed phrase (generated if not provided)"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_seed".to_string(),
            description: "Retrieve the seed phrase for a wallet created in this session.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_status".to_string(),
            description: "Get the sync status of a wallet (statusCode 3 = Ready).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_balance".to_string(),
            description: "Get the balance of a wallet (available and locked HTR in cents).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "get_wallet_addresses".to_string(),
            description: "Get the addresses of a wallet.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        McpTool {
            name: "send_from_wallet".to_string(),
            description: "Send HTR from a wallet to an address.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID to send from"
                    },
                    "address": {
                        "type": "string",
                        "description": "Destination Hathor address"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount of HTR to send"
                    }
                },
                "required": ["wallet_id", "address", "amount"]
            }),
        },
        McpTool {
            name: "close_wallet".to_string(),
            description: "Close a wallet and remove it from the wallet-headless service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        // Faucet
        McpTool {
            name: "get_faucet_balance".to_string(),
            description: "Get the balance of the fullnode's built-in wallet (faucet).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "send_from_faucet".to_string(),
            description: "Send HTR from the fullnode's built-in wallet (faucet) to an address.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "Destination Hathor address"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount of HTR to send"
                    }
                },
                "required": ["address", "amount"]
            }),
        },
        McpTool {
            name: "fund_wallet".to_string(),
            description: "Send HTR from the faucet to a wallet. Auto-determines address and reasonable amount.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID to fund"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount of HTR to send (auto-calculated if not provided)"
                    }
                },
                "required": ["wallet_id"]
            }),
        },
        // Blockchain
        McpTool {
            name: "get_blocks".to_string(),
            description: "Get recent blocks from the blockchain.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "count": {
                        "type": "integer",
                        "description": "Number of blocks to retrieve (default: 10)"
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "get_transaction".to_string(),
            description: "Get details of a specific transaction.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tx_id": {
                        "type": "string",
                        "description": "Transaction ID (hash)"
                    }
                },
                "required": ["tx_id"]
            }),
        },
        // Utilities
        McpTool {
            name: "quick_start".to_string(),
            description: "Quickly start the full environment: node, miner, and wallet service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "quick_stop".to_string(),
            description: "Stop all services: node, miner, and wallet service.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_full_status".to_string(),
            description: "Get comprehensive status of all services, balances, and active wallets.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "reset_data".to_string(),
            description: "Reset all blockchain data and stop all services. USE WITH CAUTION.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        // Nano Contracts & Blueprints
        McpTool {
            name: "list_blueprints".to_string(),
            description: "List all available blueprints on the network.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_blueprint_info".to_string(),
            description: "Get detailed information about a blueprint including its methods and arguments.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "blueprint_id": {
                        "type": "string",
                        "description": "The blueprint ID"
                    }
                },
                "required": ["blueprint_id"]
            }),
        },
        McpTool {
            name: "publish_blueprint".to_string(),
            description: "Publish an on-chain blueprint (Python source code) to the Hathor network. Requires wallet-headless running.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID to use for publishing"
                    },
                    "code": {
                        "type": "string",
                        "description": "The blueprint Python source code"
                    },
                    "address": {
                        "type": "string",
                        "description": "The caller address (must belong to the wallet)"
                    }
                },
                "required": ["wallet_id", "code", "address"]
            }),
        },
        McpTool {
            name: "create_nano_contract".to_string(),
            description: "Create (initialize) a new nano contract from a blueprint. Requires wallet-headless running.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID to use"
                    },
                    "blueprint_id": {
                        "type": "string",
                        "description": "The blueprint ID to instantiate"
                    },
                    "address": {
                        "type": "string",
                        "description": "The caller address (must belong to the wallet)"
                    },
                    "args": {
                        "type": "array",
                        "description": "Constructor arguments for the blueprint's initialize method",
                        "items": {}
                    },
                    "actions": {
                        "type": "array",
                        "description": "Actions to perform (deposit/withdrawal). Each action: {type: 'deposit'|'withdrawal', token: string, amount: number, address?: string}",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["deposit", "withdrawal"] },
                                "token": { "type": "string" },
                                "amount": { "type": "number" },
                                "address": { "type": "string" }
                            },
                            "required": ["type", "token", "amount"]
                        }
                    }
                },
                "required": ["wallet_id", "blueprint_id", "address"]
            }),
        },
        McpTool {
            name: "execute_nano_contract".to_string(),
            description: "Execute a method on an existing nano contract. Requires wallet-headless running.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wallet_id": {
                        "type": "string",
                        "description": "The wallet ID to use"
                    },
                    "nc_id": {
                        "type": "string",
                        "description": "The nano contract ID"
                    },
                    "method": {
                        "type": "string",
                        "description": "The method name to call"
                    },
                    "address": {
                        "type": "string",
                        "description": "The caller address (must belong to the wallet)"
                    },
                    "args": {
                        "type": "array",
                        "description": "Arguments for the method call",
                        "items": {}
                    },
                    "actions": {
                        "type": "array",
                        "description": "Actions to perform (deposit/withdrawal). Each action: {type: 'deposit'|'withdrawal', token: string, amount: number, address?: string}",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["deposit", "withdrawal"] },
                                "token": { "type": "string" },
                                "amount": { "type": "number" },
                                "address": { "type": "string" }
                            },
                            "required": ["type", "token", "amount"]
                        }
                    }
                },
                "required": ["wallet_id", "nc_id", "method", "address"]
            }),
        },
        McpTool {
            name: "get_nano_contract_state".to_string(),
            description: "Get the current state of a nano contract from the fullnode.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "nc_id": {
                        "type": "string",
                        "description": "The nano contract ID"
                    }
                },
                "required": ["nc_id"]
            }),
        },
        McpTool {
            name: "get_nano_contract_history".to_string(),
            description: "Get the transaction history of a nano contract.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "nc_id": {
                        "type": "string",
                        "description": "The nano contract ID"
                    }
                },
                "required": ["nc_id"]
            }),
        },
        McpTool {
            name: "get_nano_contract_logs".to_string(),
            description: "Get the execution logs for a nano contract transaction.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tx_id": {
                        "type": "string",
                        "description": "The transaction ID (hash) of the nano contract transaction"
                    }
                },
                "required": ["tx_id"]
            }),
        },
        // Service URL Configuration
        McpTool {
            name: "get_service_urls".to_string(),
            description: "Get the current service endpoint URLs (fullnode, wallet-headless, tx-mining).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "set_service_urls".to_string(),
            description: "Update service endpoint URLs at runtime. Only provided URLs are changed.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "fullnode_url": {
                        "type": "string",
                        "description": "Fullnode API URL (e.g. http://127.0.0.1:8080)"
                    },
                    "wallet_headless_url": {
                        "type": "string",
                        "description": "Wallet-headless service URL (e.g. http://localhost:8001)"
                    },
                    "tx_mining_url": {
                        "type": "string",
                        "description": "Tx-mining service URL (e.g. http://localhost:8002)"
                    }
                },
                "required": []
            }),
        },
    ]
}
