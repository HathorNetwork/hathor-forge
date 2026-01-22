# Spec: Nano Contract Integrations

## 1. Overview
Hathor Forge should provide a first-class development experience for Nano Contracts. This includes the ability to deploy contracts (initialize blueprints), list active contracts, and interact with them through a user-friendly interface.

## 2. Goals
- Provide a dedicated "Nano Contracts" section in the sidebar.
- Enable users to initialize a Nano Contract from a Blueprint (Create OCB).
- List all Nano Contracts associated with the user's wallets.
- Provide a dynamic UI to call methods on existing Nano Contracts.
- View Nano Contract state and transaction history.

## 3. User Experience (UI/UX)
- **Nano Contract Dashboard**: 
    - A searchable list of Nano Contracts.
    - Each entry shows: Contract ID, Blueprint ID, Balance, and Status.
    - "Register Contract" button to manually add a contract ID.
- **Initialization Wizard (Create Nano Contract)**:
    - Select a Blueprint (e.g., Bet, Swap, etc.).
    - Fill in initialization parameters (dynamic form based on Blueprint).
    - Select wallet for payment and signing.
    - Success screen with the new Contract ID.
- **Interaction Interface**:
    - When selecting a contract, show its current state.
    - List of available methods (from the Blueprint).
    - Clicking a method opens a form for its arguments.
    - Transaction preview before signing.

## 4. Technical Implementation

### 4.1. Backend (Rust / Tauri)
- **Tauri Commands**:
    - `get_nano_contract_state(id)`: Fetch the current state of a contract from the fullnode.
    - `get_nano_contract_history(id)`: Fetch transaction history for a contract.
    - `list_blueprints()`: List available blueprints on the network.
    - `get_blueprint_information(id)`: Get methods and arguments for a blueprint.
- **Data Persistence**:
    - Store a list of "watched" or "created" contract IDs in the local application data directory to persist them across sessions.

### 4.2. Frontend (React)
- **State Management**: Use Zustand to track the list of contracts and their states.
- **Dynamic Forms**: Implement a form builder that can take a Blueprint's argument specification and render appropriate input fields (integers, strings, addresses, etc.).
- **Integration**: Use `hathor-wallet-headless` API for signing nano contract transactions.

### 4.3. API Integration
Nano contract interactions will primarily use:
- Fullnode API (`/v1a/nano_contract/...`) for read operations.
- Wallet Headless API (`/v1a/wallet/proxy/nano_contract/...`) or similar for write operations that require signing.

## 5. Implementation Phases
1. **Phase 1: Basic UI & Listing**: Add the sidebar item and a way to manually add/track contract IDs.
2. **Phase 2: State Visualization**: Fetch and display the state of tracked contracts.
3. **Phase 3: Initialization (OCB)**: Implementation of the creation wizard.
4. **Phase 4: Method Interaction**: Dynamic form generation and transaction submission.
