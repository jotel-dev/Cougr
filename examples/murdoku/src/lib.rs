#![no_std]
extern crate alloc;

pub mod components;
pub mod systems;

use components::{Clue, PuzzleMetadata};
use cougr_core::ops::Ownable;
use cougr_core::plugin::GameApp;
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, Symbol, Vec, String};

/// Errors returned by the Murdoku smart contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PuzzleError {
    InvalidGridSize = 1,
    InvalidSuspects = 2,
    InvalidSolution = 3,
    InvalidClues = 4,
    PuzzleNotFound = 5,
    Unauthorized = 6,
}

/// Representation of a full Murdoku puzzle.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Puzzle {
    pub id: u32,
    pub creator: Address,
    pub grid_size: u32,
    pub suspects: Vec<String>,
    pub clues: Vec<Clue>,
    pub solution: Vec<u32>,
    pub metadata: PuzzleMetadata,
    pub active: bool,
}

/// Representation of a Murdoku puzzle summary (omitting the solution).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PuzzleSummary {
    pub id: u32,
    pub creator: Address,
    pub grid_size: u32,
    pub metadata: PuzzleMetadata,
    pub active: bool,
}

#[contract]
pub struct MurdokuContract;

#[contractimpl]
impl MurdokuContract {
    /// Validates and stores a new puzzle. Returns the assigned puzzle ID.
    pub fn submit_puzzle(
        env: Env,
        caller: Address,
        grid_size: u32,
        suspects: Vec<String>,
        clues: Vec<Clue>,
        solution: Vec<u32>,
        metadata: PuzzleMetadata,
    ) -> u32 {
        caller.require_auth();

        // 1. Run validation using ephemeral ECS
        let mut app = GameApp::new(&env);
        app.add_startup_system("validate_puzzle", systems::puzzle_validation_system);

        let entity_id = app.world_mut().spawn_entity();
        app.world_mut().set_typed(&env, entity_id, &components::GridSize { size: grid_size });
        app.world_mut().set_typed(&env, entity_id, &components::Suspects { list: suspects.clone() });
        app.world_mut().set_typed(&env, entity_id, &components::Clues { list: clues.clone() });
        app.world_mut().set_typed(&env, entity_id, &components::Solution { grid: solution.clone() });
        app.world_mut().set_typed(&env, entity_id, &components::Metadata { meta: metadata.clone() });

        if let Err(_) = app.run_startup(&env) {
            // Under normal circumstances, validation panics internally. But in case of startup scheduler error:
            panic_with_error!(&env, PuzzleError::InvalidSolution);
        }

        // 2. Increment the puzzle counter
        let counter_key = Symbol::new(&env, "PUZZLE_COUNT");
        let mut count: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);
        count += 1;
        env.storage().persistent().set(&counter_key, &count);
        let puzzle_id = count;

        // 3. Store the puzzle definition
        let puzzle = Puzzle {
            id: puzzle_id,
            creator: caller.clone(),
            grid_size,
            suspects,
            clues,
            solution,
            metadata,
            active: true,
        };
        let puzzle_key = (Symbol::new(&env, "PUZZLE"), puzzle_id);
        env.storage().persistent().set(&puzzle_key, &puzzle);

        // 4. Set the puzzle status
        let status_key = (Symbol::new(&env, "STATUS"), puzzle_id);
        env.storage().persistent().set(&status_key, &true);

        // 5. Initialize the ownable pattern for authorization
        let ownable_id = Symbol::new(&env, &alloc::format!("puzzle_{}", puzzle_id));
        let ownable = Ownable::new(ownable_id);
        ownable.initialize(&env, &caller).unwrap();

        puzzle_id
    }

    /// Returns the full puzzle definition including clues and solution.
    pub fn get_puzzle(env: Env, puzzle_id: u32) -> Puzzle {
        let puzzle_key = (Symbol::new(&env, "PUZZLE"), puzzle_id);
        let mut puzzle: Puzzle = match env.storage().persistent().get(&puzzle_key) {
            Some(p) => p,
            None => panic_with_error!(&env, PuzzleError::PuzzleNotFound),
        };
        let status_key = (Symbol::new(&env, "STATUS"), puzzle_id);
        let active = env.storage().persistent().get(&status_key).unwrap_or(false);
        puzzle.active = active;
        puzzle
    }

    /// Returns a paginated list of puzzle summaries (no solution field).
    pub fn list_puzzles(env: Env, offset: u32, limit: u32) -> Vec<PuzzleSummary> {
        let counter_key = Symbol::new(&env, "PUZZLE_COUNT");
        let total: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);

        let mut list = Vec::new(&env);
        if offset >= total {
            return list;
        }

        let start = offset + 1;
        let end = (offset + limit).min(total);

        for id in start..=end {
            let puzzle_key = (Symbol::new(&env, "PUZZLE"), id);
            if let Some(puzzle) = env.storage().persistent().get::<_, Puzzle>(&puzzle_key) {
                let status_key = (Symbol::new(&env, "STATUS"), id);
                let active = env.storage().persistent().get(&status_key).unwrap_or(false);
                list.push_back(PuzzleSummary {
                    id: puzzle.id,
                    creator: puzzle.creator,
                    grid_size: puzzle.grid_size,
                    metadata: puzzle.metadata,
                    active,
                });
            }
        }
        list
    }

    /// Returns the total number of submitted puzzles.
    pub fn get_puzzle_count(env: Env) -> u32 {
        let counter_key = Symbol::new(&env, "PUZZLE_COUNT");
        env.storage().persistent().get(&counter_key).unwrap_or(0)
    }

    /// Allows the original creator to deactivate their puzzle. Uses ops::Ownable pattern for authorization.
    pub fn deactivate_puzzle(env: Env, caller: Address, puzzle_id: u32) {
        caller.require_auth();

        let ownable_id = Symbol::new(&env, &alloc::format!("puzzle_{}", puzzle_id));
        let ownable = Ownable::new(ownable_id);

        if let Err(_) = ownable.require_owner(&env, &caller) {
            panic_with_error!(&env, PuzzleError::Unauthorized);
        }

        let status_key = (Symbol::new(&env, "STATUS"), puzzle_id);
        if !env.storage().persistent().has(&status_key) {
            panic_with_error!(&env, PuzzleError::PuzzleNotFound);
        }

        env.storage().persistent().set(&status_key, &false);

        // Update the cached active field inside the persistent Puzzle definition
        let puzzle_key = (Symbol::new(&env, "PUZZLE"), puzzle_id);
        if let Some(mut puzzle) = env.storage().persistent().get::<_, Puzzle>(&puzzle_key) {
            puzzle.active = false;
            env.storage().persistent().set(&puzzle_key, &puzzle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String, Vec};

    fn make_valid_puzzle(env: &Env) -> (u32, Vec<String>, Vec<Clue>, Vec<u32>, PuzzleMetadata) {
        let grid_size = 4;
        let mut suspects = Vec::new(env);
        suspects.push_back(String::from_str(env, "Alice"));
        suspects.push_back(String::from_str(env, "Bob"));
        suspects.push_back(String::from_str(env, "Charlie"));
        suspects.push_back(String::from_str(env, "David"));

        let mut clues = Vec::new(env);
        clues.push_back(Clue {
            row: 0,
            col: 0,
            suspect_idx: 1,
        });
        clues.push_back(Clue {
            row: 1,
            col: 1,
            suspect_idx: 3,
        });

        // Valid Latin square:
        // [1, 2, 3, 4]
        // [2, 3, 4, 1]
        // [3, 4, 1, 2]
        // [4, 1, 2, 3]
        let mut solution = Vec::new(env);
        solution.push_back(1);
        solution.push_back(2);
        solution.push_back(3);
        solution.push_back(4);

        solution.push_back(2);
        solution.push_back(3);
        solution.push_back(4);
        solution.push_back(1);

        solution.push_back(3);
        solution.push_back(4);
        solution.push_back(1);
        solution.push_back(2);

        solution.push_back(4);
        solution.push_back(1);
        solution.push_back(2);
        solution.push_back(3);

        let metadata = PuzzleMetadata {
            name: String::from_str(env, "Classic Case"),
            difficulty: String::from_str(env, "Easy"),
        };

        (grid_size, suspects, clues, solution, metadata)
    }

    #[test]
    fn test_submit_valid_puzzle() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, clues, solution, metadata) = make_valid_puzzle(&env);

        let id = client.submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        assert_eq!(id, 1);
        assert_eq!(client.get_puzzle_count(), 1);

        let puzzle = client.get_puzzle(&1);
        assert_eq!(puzzle.id, 1);
        assert_eq!(puzzle.creator, creator);
        assert_eq!(puzzle.grid_size, 4);
        assert_eq!(puzzle.suspects.len(), 4);
        assert_eq!(puzzle.clues.len(), 2);
        assert_eq!(puzzle.solution.len(), 16);
        assert_eq!(puzzle.active, true);
    }

    #[test]
    fn test_submit_invalid_grid_size() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (_, suspects, clues, solution, metadata) = make_valid_puzzle(&env);

        let result = client.try_submit_puzzle(&creator, &3, &suspects, &clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidGridSize as u32);
        } else {
            panic!("Expected InvalidGridSize contract error");
        }
    }

    #[test]
    fn test_submit_invalid_suspects_length() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, mut suspects, clues, solution, metadata) = make_valid_puzzle(&env);
        suspects.pop_back(); // length 3 instead of 4

        let result = client.try_submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidSuspects as u32);
        } else {
            panic!("Expected InvalidSuspects contract error");
        }
    }

    #[test]
    fn test_submit_empty_suspect_name() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, mut suspects, clues, solution, metadata) = make_valid_puzzle(&env);
        suspects.set(1, String::from_str(&env, "")); // empty name

        let result = client.try_submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidSuspects as u32);
        } else {
            panic!("Expected InvalidSuspects contract error");
        }
    }

    #[test]
    fn test_submit_invalid_solution_length() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, clues, mut solution, metadata) = make_valid_puzzle(&env);
        solution.pop_back(); // length 15 instead of 16

        let result = client.try_submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidSolution as u32);
        } else {
            panic!("Expected InvalidSolution contract error");
        }
    }

    #[test]
    fn test_submit_invalid_latin_square() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, clues, mut solution, metadata) = make_valid_puzzle(&env);
        solution.set(1, 1); // Row 0 is now [1, 1, 3, 4] -> duplicate 1

        let result = client.try_submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidSolution as u32);
        } else {
            panic!("Expected InvalidSolution contract error");
        }
    }

    #[test]
    fn test_submit_empty_clues() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, _, solution, metadata) = make_valid_puzzle(&env);
        let empty_clues = Vec::new(&env);

        let result = client.try_submit_puzzle(&creator, &grid_size, &suspects, &empty_clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidClues as u32);
        } else {
            panic!("Expected InvalidClues contract error");
        }
    }

    #[test]
    fn test_submit_invalid_clue_coordinate() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, mut clues, solution, metadata) = make_valid_puzzle(&env);
        clues.push_back(Clue {
            row: 4, // Out of bounds for size 4
            col: 0,
            suspect_idx: 1,
        });

        let result = client.try_submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidClues as u32);
        } else {
            panic!("Expected InvalidClues contract error");
        }
    }

    #[test]
    fn test_submit_clue_mismatch_with_solution() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, mut clues, solution, metadata) = make_valid_puzzle(&env);
        clues.push_back(Clue {
            row: 0,
            col: 1,
            suspect_idx: 4, // Solution at (0,1) is 2, not 4
        });

        let result = client.try_submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::InvalidClues as u32);
        } else {
            panic!("Expected InvalidClues contract error");
        }
    }

    #[test]
    fn test_list_puzzles_and_pagination() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, clues, solution, metadata) = make_valid_puzzle(&env);

        client.submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        client.submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);
        client.submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);

        assert_eq!(client.get_puzzle_count(), 3);

        let list_all = client.list_puzzles(&0, &10);
        assert_eq!(list_all.len(), 3);
        assert_eq!(list_all.get(0).unwrap().id, 1);
        assert_eq!(list_all.get(1).unwrap().id, 2);
        assert_eq!(list_all.get(2).unwrap().id, 3);

        let list_page = client.list_puzzles(&1, &1);
        assert_eq!(list_page.len(), 1);
        assert_eq!(list_page.get(0).unwrap().id, 2);
    }

    #[test]
    fn test_deactivate_puzzle_authorization() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MurdokuContract, ());
        let client = MurdokuContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let (grid_size, suspects, clues, solution, metadata) = make_valid_puzzle(&env);

        client.submit_puzzle(&creator, &grid_size, &suspects, &clues, &solution, &metadata);

        // A non-creator tries to deactivate and gets rejected
        let intruder = Address::generate(&env);
        let result = client.try_deactivate_puzzle(&intruder, &1);
        assert!(result.is_err());
        if let Err(soroban_sdk::InvokeError::ContractError(code)) = result {
            assert_eq!(code, PuzzleError::Unauthorized as u32);
        } else {
            panic!("Expected Unauthorized contract error");
        }

        // Creator deactivates successfully
        client.deactivate_puzzle(&creator, &1);

        let puzzle = client.get_puzzle(&1);
        assert_eq!(puzzle.active, false);

        let summaries = client.list_puzzles(&0, &1);
        assert_eq!(summaries.get(0).unwrap().active, false);
    }
}
