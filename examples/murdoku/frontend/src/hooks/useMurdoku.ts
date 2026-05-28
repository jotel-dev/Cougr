import { useState, useEffect, useCallback, useRef } from 'react';
import {
  MOCK_PUZZLES,
  MOCK_SUMMARIES,
} from '../data/mockData';
import type {
  PuzzleSummaryWithSolvers,
  ContractPuzzle,
  PlayerState,
  MoveResult,
} from '../types';

const PAGE_SIZE = 6;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function mapMockPuzzleToContractPuzzle(id: string): ContractPuzzle | null {
  const puzzle = MOCK_PUZZLES.find((p) => p.id === id);
  if (!puzzle) return null;
  return {
    id: puzzle.id,
    title: puzzle.title,
    description: puzzle.description,
    gridSize: puzzle.gridSize,
    difficulty: puzzle.difficulty,
    suspects: puzzle.suspects,
    clues: puzzle.clues,
    solution: puzzle.solution,
    creatorAddress: puzzle.creatorAddress,
    active: true,
  };
}

function mapMockSummaryToContractSummary(
  summary: (typeof MOCK_SUMMARIES)[number],
): PuzzleSummaryWithSolvers {
  return {
    id: summary.id,
    title: summary.title,
    gridSize: summary.gridSize,
    difficulty: summary.difficulty,
    totalSolvers: Math.floor(Math.random() * 50),
    active: true,
    creatorAddress: summary.creatorAddress,
  };
}

// ─── usePuzzleList ────────────────────────────────────────────────────────────

interface UsePuzzleListReturn {
  puzzles: PuzzleSummaryWithSolvers[];
  loading: boolean;
  error: string | null;
  hasMore: boolean;
  loadMore: () => void;
}

export function usePuzzleList(limit: number = PAGE_SIZE): UsePuzzleListReturn {
  const [puzzles, setPuzzles] = useState<PuzzleSummaryWithSolvers[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    const fetchPuzzles = async () => {
      try {
        setLoading(true);
        setError(null);
        await delay(600);

        const total = MOCK_SUMMARIES.length;
        const start = offset;
        const end = Math.min(offset + limit, total);
        const page = MOCK_SUMMARIES.slice(start, end).map(mapMockSummaryToContractSummary);

        if (mountedRef.current) {
          setPuzzles((prev) => {
            const existingIds = new Set(prev.map((p) => p.id));
            const newPuzzles = page.filter((p) => !existingIds.has(p.id));
            return [...prev, ...newPuzzles];
          });
          setHasMore(end < total);
          setLoading(false);
        }
      } catch (e) {
        if (mountedRef.current) {
          setError('Failed to fetch puzzle catalog');
          setLoading(false);
        }
      }
    };

    fetchPuzzles();

    return () => {
      mountedRef.current = false;
    };
  }, [offset, limit]);

  const loadMore = useCallback(() => {
    setOffset((prev) => prev + limit);
  }, [limit]);

  return { puzzles, loading, error, hasMore, loadMore };
}

// ─── usePuzzle ────────────────────────────────────────────────────────────────

interface UsePuzzleReturn {
  puzzle: ContractPuzzle | null;
  loading: boolean;
  error: string | null;
}

export function usePuzzle(puzzleId: string): UsePuzzleReturn {
  const [puzzle, setPuzzle] = useState<ContractPuzzle | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    const fetchPuzzle = async () => {
      try {
        setLoading(true);
        setError(null);
        await delay(400);

        const result = mapMockPuzzleToContractPuzzle(puzzleId);

        if (mounted) {
          if (!result) {
            setError('Puzzle not found');
            setPuzzle(null);
          } else {
            setPuzzle(result);
          }
          setLoading(false);
        }
      } catch (e) {
        if (mounted) {
          setError('Failed to load puzzle');
          setLoading(false);
        }
      }
    };

    if (puzzleId) {
      fetchPuzzle();
    }

    return () => {
      mounted = false;
    };
  }, [puzzleId]);

  return { puzzle, loading, error };
}

// ─── usePlayerState ───────────────────────────────────────────────────────────

interface UsePlayerStateReturn {
  playerState: PlayerState | null;
  loading: boolean;
  error: string | null;
  hasSession: boolean;
  refresh: () => void;
}

const MOCK_PLAYER_STATES: Record<string, PlayerState> = {};

export function usePlayerState(
  puzzleId: string,
  _gridSize: number,
): UsePlayerStateReturn {
  const [playerState, setPlayerState] = useState<PlayerState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasSession, setHasSession] = useState(false);

  const fetchState = useCallback(() => {
    let mounted = true;
    const fn = async () => {
      try {
        setLoading(true);
        setError(null);
        await delay(300);

        const existing = MOCK_PLAYER_STATES[puzzleId];

        if (mounted) {
          if (existing) {
            setPlayerState(existing);
            setHasSession(true);
          } else {
            setPlayerState(null);
            setHasSession(false);
          }
          setLoading(false);
        }
      } catch (e) {
        if (mounted) {
          setError('Failed to load player state');
          setLoading(false);
        }
      }
    };
    fn();
    return () => {
      mounted = false;
    };
  }, [puzzleId]);

  useEffect(() => {
    const cleanup = fetchState();
    return cleanup;
  }, [fetchState]);

  useEffect(() => {
    if (!hasSession) return;
    const interval = setInterval(() => {
      fetchState();
    }, 3000);
    return () => clearInterval(interval);
  }, [hasSession, fetchState]);

  const refresh = useCallback(() => {
    fetchState();
  }, [fetchState]);

  return { playerState, loading, error, hasSession, refresh };
}

// ─── useStartGame ─────────────────────────────────────────────────────────────

interface UseStartGameReturn {
  startGame: () => Promise<void>;
  loading: boolean;
  error: string | null;
}

export function useStartGame(
  puzzleId: string,
  gridSize: number,
): UseStartGameReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startGame = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      await delay(500);

      const emptyCells = Array.from({ length: gridSize * gridSize }, () => ({
        suspectId: null,
      }));

      MOCK_PLAYER_STATES[puzzleId] = {
        cells: emptyCells,
        moveCount: 0,
        solved: false,
      };
    } catch (e) {
      setError('Failed to start game session');
    } finally {
      setLoading(false);
    }
  }, [puzzleId, gridSize]);

  return { startGame, loading, error };
}

// ─── usePlaceSuspect ──────────────────────────────────────────────────────────

interface UsePlaceSuspectReturn {
  placeSuspect: (
    row: number,
    col: number,
    suspectId: number,
  ) => Promise<MoveResult>;
  loading: boolean;
  error: string | null;
}

export function usePlaceSuspect(
  puzzleId: string,
  gridSize: number,
): UsePlaceSuspectReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const placeSuspect = useCallback(
    async (row: number, col: number, suspectId: number): Promise<MoveResult> => {
      try {
        setLoading(true);
        setError(null);
        await delay(400);

        const state = MOCK_PLAYER_STATES[puzzleId];
        if (!state) {
          throw new Error('No active game session');
        }

        if (state.solved) {
          return 'GameAlreadySolved';
        }

        if (row < 0 || row >= gridSize || col < 0 || col >= gridSize) {
          return 'InvalidCoordinates';
        }

        const cellIndex = row * gridSize + col;
        const cell = state.cells[cellIndex];

        if (cell.suspectId !== null) {
          return 'CellOccupied';
        }

        for (let c = 0; c < gridSize; c++) {
          const idx = row * gridSize + c;
          if (idx !== cellIndex && state.cells[idx].suspectId === suspectId) {
            return 'RowConflict';
          }
        }

        for (let r = 0; r < gridSize; r++) {
          const idx = r * gridSize + col;
          if (idx !== cellIndex && state.cells[idx].suspectId === suspectId) {
            return 'ColConflict';
          }
        }

        state.cells[cellIndex] = { suspectId };
        state.moveCount += 1;

        const puzzle = mapMockPuzzleToContractPuzzle(puzzleId);
        if (puzzle) {
          const allFilled = state.cells.every((c) => c.suspectId !== null);
          const matchesSolution = state.cells.every(
            (c, i) => c.suspectId === puzzle.solution[i],
          );
          if (allFilled && matchesSolution) {
            state.solved = true;
          }
        }

        return 'Ok';
      } catch (e) {
        const msg = e instanceof Error ? e.message : 'Failed to place suspect';
        setError(msg);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [puzzleId, gridSize],
  );

  return { placeSuspect, loading, error };
}

// ─── useRemoveSuspect ─────────────────────────────────────────────────────────

interface UseRemoveSuspectReturn {
  removeSuspect: (row: number, col: number) => Promise<MoveResult>;
  loading: boolean;
  error: string | null;
}

export function useRemoveSuspect(
  puzzleId: string,
  gridSize: number,
): UseRemoveSuspectReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const removeSuspect = useCallback(
    async (row: number, col: number): Promise<MoveResult> => {
      try {
        setLoading(true);
        setError(null);
        await delay(300);

        const state = MOCK_PLAYER_STATES[puzzleId];
        if (!state) {
          throw new Error('No active game session');
        }

        if (state.solved) {
          return 'GameAlreadySolved';
        }

        if (row < 0 || row >= gridSize || col < 0 || col >= gridSize) {
          return 'InvalidCoordinates';
        }

        const cellIndex = row * gridSize + col;
        const cell = state.cells[cellIndex];

        if (cell.suspectId === null) {
          return 'InvalidCoordinates';
        }

        state.cells[cellIndex] = { suspectId: null };
        state.moveCount += 1;
        state.solved = false;

        return 'Ok';
      } catch (e) {
        const msg = e instanceof Error ? e.message : 'Failed to remove suspect';
        setError(msg);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [puzzleId, gridSize],
  );

  return { removeSuspect, loading, error };
}

// ─── useIsSolved ──────────────────────────────────────────────────────────────

export function useIsSolved(puzzleId: string): boolean {
  const [solved, setSolved] = useState(false);

  useEffect(() => {
    const state = MOCK_PLAYER_STATES[puzzleId];
    if (state) {
      setSolved(state.solved);
    }
  }, [puzzleId]);

  return solved;
}
