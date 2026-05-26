# Murdoku

Murdoku is a murder mystery logic puzzle built with the [Cougr](../../README.md) ECS framework on Stellar Soroban. It demonstrates how to combine Entity Component System (ECS) game state, Pollar social logins, and session key authorization for smooth, gasless player interaction.

## Purpose and Pattern

Murdoku is an on-chain logic puzzle that combines Sudoku-style Latin square constraints with murder mystery storytelling. Players place suspects (e.g., characters, weapons, rooms) on a grid while satisfying constraints provided by clues (e.g., "The suspect with the knife is not in the same row as the butler"). 

This example showcases the canonical reference architecture for a full-stack game within the Cougr ecosystem:
* **Entity Component System (ECS)**: Separation of game state, input validation, puzzle constraint checks, and win conditions using the Cougr ECS engine.
* **Social Authentication & Session Keys**: Account abstraction utilizing Pollar for embedded wallet logins and scoped session key creation (`SessionBuilder`) to support non-intrusive gameplay transactions.
* **ZK Readiness**: Architectural scaffolding for Zero-Knowledge (ZK) enhancements. In the v1 release, solutions and clue validations are stored in plaintext. In the subsequent v2 enhancement (tracked separately), the grid is submitted as a cryptographic commitment, and moves are verified using Groth16 zk-SNARK proofs generated client-side to prevent front-running and keep solutions private.

---

## Architecture Overview

Murdoku follows a decoupled design where all gameplay rules and state changes are handled by the Soroban contract, while the user interface and wallet management are facilitated by a React web app.

```
Browser (Vite + React)
  └── Pollar (@pollar/react)       — embedded wallet, social login
  └── contract.ts                  — Stellar SDK contract client
        │
        ▼
  Soroban Contract (examples/murdoku/src/)
  ├── lib.rs       — entrypoints & GameApp wiring
  ├── components.rs — ECS components (Board, Suspect, GameState)
  ├── systems.rs   — validation, move execution, and completion systems
  ├── types.rs     — domain types (Clues, Suspects, GridConfiguration)
  └── auth.rs      — session keys, CougrAccount wiring
        │
        ▼
  Stellar Testnet (Soroban RPC)
```

The React frontend embeds the Pollar SDK for OAuth/social logins. Pollar creates a temporary keypair representing a session key. The frontend uses the session key to sign game moves (e.g., `place_suspect`). The Soroban contract verifies that the session key is valid and authorized to act on behalf of the player using the `authorize_with_fallback` module.

---

## Public Contract API

Below is the entrypoint surface defined in the `#[contractimpl]` block of the Murdoku smart contract.

| Function | Parameters | Return Type | Description |
|---|---|---|---|
| `init_game` | `env: Env`, `admin: Address` | `()` | Initializes the contract state and registers the game administrator. |
| `submit_puzzle` | `env: Env`, `creator: Address`, `size: u32`, `solution: Vec<u32>`, `clues: Vec<Clue>` | `BytesN<32>` | Registers a new puzzle in the catalog. Validates that the solution is a valid Latin square of the specified size and that the clues are consistent. Returns the unique puzzle ID. |
| `list_puzzles` | `env: Env` | `Vec<PuzzleSummary>` | Returns a list of summaries of all registered puzzles in the catalog. |
| `get_puzzle` | `env: Env`, `id: BytesN<32>` | `Puzzle` | Retrieves the grid configuration and clues for a specific puzzle ID. Excludes the solution. |
| `start_game` | `env: Env`, `player: Address`, `puzzle_id: BytesN<32>` | `()` | Spawns a new active game session for the player. Triggers the initialization systems. |
| `place_suspect` | `env: Env`, `session_key: Address`, `row: u32`, `col: u32`, `suspect_id: u32` | `()` | Places a suspect at the specified cell coordinates. Validates constraints and evaluates if the puzzle has been solved. Authenticated via session key. |
| `is_solved` | `env: Env`, `player: Address` | `bool` | Returns `true` if the player's active puzzle has been successfully solved. |
| `register_passkey` | `env: Env`, `player: Address`, `pubkey: BytesN<65>` | `()` | Registers a secp256r1 public key (e.g. passkey) for the player. |
| `authenticate_and_create_session` | `env: Env`, `player: Address`, `signature: Bytes`, `challenge: BytesN<32>`, `duration: u64` | `Address` | Verifies a passkey signature and returns a scoped session key address. |

---

## Storage Model

Murdoku uses Soroban's state storage model (Instance, Persistent, and Temporary) to optimize gas costs and storage lifetimes.

| Storage Type | Data Kept | Lifetime | Rationale |
|---|---|---|---|
| **Persistent** | Registered Puzzle catalog (by ID), Creator profiles, Passkey credentials | Indefinite | Puzzle templates and user credentials must persist forever across sessions and are read-heavy. |
| **Instance** | Active Player Game session, ECS World state (`SimpleWorld`), Admin configs | Extended (Renewed on play) | The active board state must persist during active play but can be garbage-collected or archived if the player abandons the game. |
| **Temporary** | Active Session Key tokens, cryptographic challenges | Short-term (Expires in blocks) | Session authorization keys only need to last for the duration of the play session and should expire automatically to free state space. |

---

## Main Gameplay Flow

### Play Flow
1. **Connect Wallet**: The player logs into the frontend via Pollar using an email or social login. Pollar provisions a Stellar account and prepares a local session key.
2. **Browse Catalog**: The frontend calls `list_puzzles` to fetch and render the list of available murder mystery puzzles.
3. **Open Puzzle**: The player selects a puzzle. The frontend calls `get_puzzle` to retrieve the clues and grid dimensions, then calls `start_game` to initialize the ECS world state for this puzzle.
4. **Place Suspects**: The player drags and drops suspects (e.g., characters, weapons) onto grid cells. Each placement calls `place_suspect`. This transaction is signed by the local session key and bypasses manual wallet confirmations.
5. **On-chain Validation**: The contract executes the validation system:
   - Verifies the cell is editable.
   - Enforces Latin square constraints (no duplicate suspects in the same row or column).
   - Validates placement against the puzzle's clues.
6. **Completion**: If all cells are filled correctly, the contract marks the game status as solved. `is_solved` returns `true`, and the frontend displays a victory screen.

### Create Flow
1. **Connect Wallet**: The creator authenticates on the creator portal via Pollar.
2. **Configure Puzzle**: The creator designs a puzzle:
   - Selects grid size (4x4 or 5x5).
   - Enters the full solution grid.
   - Defines the clues (e.g., cell constraints, adjacency rules).
3. **Submit Puzzle**: The creator clicks "Publish", triggering a call to `submit_puzzle`.
4. **Contract Invariant Check**: The contract validates:
   - The solution grid constitutes a mathematically valid Latin square.
   - The provided clues do not conflict with the solution.
5. **Publishing**: If checks pass, the contract generates a unique puzzle ID (SHA-256 hash of solution and clues), stores the puzzle templates in persistent storage, and adds it to the list of active games.

---

## Cougr APIs Used

Murdoku is built on the core APIs of the Cougr framework.

| API / Module | Used For | Location |
|---|---|---|
| `GameApp` | Orchestrates systems execution flow during suspect placement and puzzle registration. | `src/lib.rs` |
| `ScheduleStage` | Ensures that validation systems execute strictly before update systems, and completion systems execute last. | `src/lib.rs` |
| `SimpleWorld` | Stores the active ECS state including entity components (e.g. Board, MoveCount). | `src/lib.rs`, `src/systems.rs` |
| `impl_component!` | Macros defining serialized game components like `BoardComponent` and `GameStatusComponent`. | `src/components.rs` |
| `auth::SessionBuilder` | Builds scoped session keys containing specific permissions (e.g., only allowing `place_suspect`). | `src/auth.rs` |
| `auth::authorize_with_fallback` | Authorizes transactions using either the active session key or the player's direct signature. | `src/auth.rs` |
| `ops::Ownable` | Restricts contract initialization and admin parameters to the contract owner. | `src/lib.rs` |

---

## Frontend Setup

### 1. Requirements
* Node.js v18 or later
* Stellar CLI installed and configured

### 2. Run Locally
Navigate to the frontend directory:
```bash
cd examples/murdoku/frontend
```

Copy the environment template:
```bash
cp .env.example .env
```

Open `.env` and set the variables:
* `VITE_POLLAR_API_KEY`: Obtain this from the Pollar Developer Console by registering an application.
* `VITE_CONTRACT_ID`: The deployed contract ID from your testnet deployment (see below).

Install dependencies and run the Vite development server:
```bash
npm install
npm run dev
```

The frontend will run locally on `http://localhost:5173`.

---

## Contract Build and Test Commands

Execute the following commands from the `examples/murdoku` directory to format, lint, test, and build the contract.

```bash
cd examples/murdoku
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
stellar contract build
```

---

## Deploying to Testnet

To deploy and test the contract manually on Stellar Testnet, use the following commands:

```bash
# 1. Generate an identity for deployment
stellar keys generate murdoku_deployer

# 2. Fund the identity using Friendbot
stellar keys fund murdoku_deployer --network testnet

# 3. Deploy the compiled WASM to Testnet
CONTRACT_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/murdoku.wasm \
  --network testnet \
  --source murdoku_deployer)

# 4. Initialize the contract game registry
stellar contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source murdoku_deployer \
  -- init_game \
  --admin murdoku_deployer

# 5. Submit a minimal 4x4 placeholder puzzle to the catalog
# (Solution grid contains flat representation of a 4x4 Latin square)
stellar contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source murdoku_deployer \
  -- submit_puzzle \
  --creator murdoku_deployer \
  --size 4 \
  --solution '[1,2,3,4,2,3,4,1,3,4,1,2,4,1,2,3]' \
  --clues '[]'
```

---

## Known Limitations

* **Plaintext Solutions**: In the v1 release, solutions and clue validations are stored in plaintext on-chain. ZK commit-reveal is tracked as a separate v2 enhancement.
* **Grid Sizes**: Grid sizes are constrained to 4x4 and 5x5 to maintain low transaction complexity and stay within Soroban CPU/memory limits.
* **Immutable Puzzles**: Puzzles cannot be edited or deleted once submitted to prevent breaking active player games.
* **No Client Proof Generation**: ZK mode does not generate proofs in the client during play in v1.
* **Development Circuit Setup**: The trusted setup used for ZK verification circuits in development is not production-safe and must be regenerated with a ceremony before launch.

---

## Classification

Marked as **Canonical**. Murdoku is the reference full-stack example for Cougr, showcasing how to build games that combine ECS, session-key-based social logins (via Pollar), and multi-step transaction authorization. Use this as the template for production-grade full-stack game contracts.
