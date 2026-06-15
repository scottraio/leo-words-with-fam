// Plain helpers for the Words With Fam UI: board metadata, scoring hints, and a
// typed wrapper around leo's shared `request` client. No React in here.

declare global {
  interface Window {
    __LEO_SHARED__: any;
  }
}

const shared = window.__LEO_SHARED__;
const BASE = "/p/words-with-fam/api";

function get<T>(path: string): Promise<T> {
  return shared.api.request<T>(BASE + path);
}
function post<T>(path: string, body?: unknown): Promise<T> {
  return shared.api.request<T>(BASE + path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
}

// ── types ──

export interface Player {
  id: string;
  name: string;
  color: string;
  avatar: string;
}
export interface PlayerView {
  id: string;
  name: string;
  color: string;
  score: number;
  rack_count: number;
  resigned: boolean;
}
export type Cell = { letter: string; blank: boolean } | null;
export interface GameView {
  id: string;
  name: string;
  status: string;
  board: Cell[][];
  players: PlayerView[];
  current_player_id: string;
  bag_remaining: number;
  move_count: number;
  finished: boolean;
  winner_id: string | null;
  your_rack: string[];
  your_player_id: string | null;
}
export interface GameSummary {
  id: string;
  name: string;
  status: string;
  current_player_id: string;
  current_player_name: string;
  players: PlayerView[];
  winner_id: string | null;
  updated_at: string;
}
export interface WordScore {
  text: string;
  points: number;
}
export interface MoveResponse {
  outcome?: { score: number; words: WordScore[]; game_over: boolean };
  game: GameView;
}
export interface MoveRecord {
  id: string;
  player_id: string;
  player_name: string;
  move_no: number;
  kind: string;
  words: WordScore[];
  score: number;
  created_at: string;
}
export interface LeaderRow {
  player_id: string;
  name: string;
  color: string;
  games_played: number;
  games_won: number;
  total_points: number;
  best_word: string;
  best_word_score: number;
}

export interface Placement {
  row: number;
  col: number;
  letter: string;
  is_blank: boolean;
}

export const wf = {
  me: (): string | null => shared.api.getCurrentUserId(),
  players: () => get<Player[]>("/players"),
  games: () => get<GameSummary[]>("/games"),
  game: (id: string) => get<GameView>(`/games/${id}`),
  history: (id: string) => get<MoveRecord[]>(`/games/${id}/moves`),
  leaderboard: () => get<LeaderRow[]>("/leaderboard"),
  create: (name: string, opponent_ids: string[]) =>
    post<GameView>("/games", { name, opponent_ids }),
  play: (id: string, placements: Placement[]) =>
    post<MoveResponse>(`/games/${id}/moves`, { placements }),
  swap: (id: string, tiles: string[]) => post<MoveResponse>(`/games/${id}/swap`, { tiles }),
  pass: (id: string) => post<MoveResponse>(`/games/${id}/pass`),
  resign: (id: string) => post<MoveResponse>(`/games/${id}/resign`),
};

// ── board metadata (mirrors wordsfam-core constants) ──

export type Premium = "TW" | "TL" | "DW" | "DL" | "START" | null;

const TW = [[0,3],[0,11],[3,0],[3,14],[11,0],[11,14],[14,3],[14,11]];
const TL = [[0,6],[0,8],[2,7],[5,5],[5,9],[6,0],[6,14],[7,2],[7,12],[8,0],[8,14],[9,5],[9,9],[12,7],[14,6],[14,8]];
const DW = [[4,4],[4,10],[6,6],[6,8],[8,6],[8,8],[10,4],[10,10]];
const DL = [[1,5],[1,9],[2,2],[2,12],[3,3],[3,11],[5,1],[5,13],[9,1],[9,13],[11,3],[11,11],[12,2],[12,12],[13,5],[13,9]];

const PREMIUM: Premium[][] = (() => {
  const g: Premium[][] = Array.from({ length: 15 }, () => Array<Premium>(15).fill(null));
  for (const [r, c] of TW) g[r][c] = "TW";
  for (const [r, c] of TL) g[r][c] = "TL";
  for (const [r, c] of DW) g[r][c] = "DW";
  for (const [r, c] of DL) g[r][c] = "DL";
  g[7][7] = "START";
  return g;
})();

export function premiumAt(r: number, c: number): Premium {
  return PREMIUM[r]?.[c] ?? null;
}

export const PREMIUM_LABEL: Record<string, string> = { TW: "3W", TL: "3L", DW: "2W", DL: "2L" };
export const PREMIUM_COLOR: Record<string, string> = {
  TW: "#c2410c",
  TL: "#1d4ed8",
  DW: "#be185d",
  DL: "#0d9488",
  START: "#7c3aed",
};

const POINTS: Record<string, number> = {
  A:1,E:1,I:1,O:1,R:1,S:1,T:1, D:2,L:2,N:2,U:2, G:3,H:3,
  B:4,C:4,F:4,M:4,P:4,W:4,Y:4, K:5,V:5, X:8, J:10,Q:10,Z:10,
};
export function letterPoints(letter: string, blank = false): number {
  if (blank) return 0;
  return POINTS[(letter || "").toUpperCase()] ?? 0;
}

export const TILE_BG = "#f5e6c8";
export const TILE_INK = "#1b1206";
export const BOARD_BG = "#0d1117";
export const CELL_BG = "#1b2430";
