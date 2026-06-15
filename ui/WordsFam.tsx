/* @jsxRuntime classic */
// Words With Fam — embedded leo UI. Fills its panel 100% with no page scroll;
// tiles are dragged from the rack onto the board (pointer events → mouse+touch).

const React = window.__LEO_SHARED__.React;
const { useState, useEffect, useMemo, useCallback, useRef } = React;
const HeroUI = window.__LEO_SHARED__.HeroUI;
const { Button, Chip, Spinner, Avatar } = HeroUI;

import {
  wf,
  premiumAt,
  PREMIUM_LABEL,
  PREMIUM_COLOR,
  letterPoints,
  TILE_BG,
  TILE_INK,
  BOARD_BG,
  CELL_BG,
} from "./lib";
import type { GameView, GameSummary, Player, MoveRecord, LeaderRow, Placement } from "./lib";

type Screen =
  | { name: "home" }
  | { name: "new" }
  | { name: "game"; id: string }
  | { name: "leaderboard" };

const fill: any = { width: "100%", height: "100%", boxSizing: "border-box" };
const colFill: any = { ...fill, display: "flex", flexDirection: "column", overflow: "hidden" };

export default function WordsFam() {
  const me = wf.me();
  const [screen, setScreen] = useState<Screen>({ name: "home" });

  if (!me) {
    return <div style={{ ...colFill, alignItems: "center", justifyContent: "center", color: "#9aa4b2" }}>Sign in to leo to play.</div>;
  }

  return (
    <div style={{ ...colFill, color: "#e6edf3", userSelect: "none" }}>
      <AppBar
        screen={screen.name}
        onHome={() => setScreen({ name: "home" })}
        onLeaderboard={() => setScreen({ name: "leaderboard" })}
      />
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
        {screen.name === "home" && <Home me={me} onOpen={(id) => setScreen({ name: "game", id })} onNew={() => setScreen({ name: "new" })} />}
        {screen.name === "new" && <NewGame me={me} onCreated={(id) => setScreen({ name: "game", id })} onCancel={() => setScreen({ name: "home" })} />}
        {screen.name === "game" && <GameScreen me={me} gameId={screen.id} />}
        {screen.name === "leaderboard" && <Leaderboard />}
      </div>
    </div>
  );
}

function AppBar({ screen, onHome, onLeaderboard }: { screen: string; onHome: () => void; onLeaderboard: () => void }) {
  return (
    <div style={{ flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "space-between", padding: "6px 12px", borderBottom: "1px solid rgba(255,255,255,.06)" }}>
      <button onClick={onHome} style={{ background: "none", border: "none", cursor: "pointer" }}>
        <span style={{ fontSize: 18, fontWeight: 800, color: "#e6edf3", letterSpacing: -0.4 }}>
          Words With <span style={{ color: "#f5a623" }}>Fam</span>
        </span>
      </button>
      <div style={{ display: "flex", gap: 6 }}>
        {screen !== "home" && <Button size="sm" variant="light" onPress={onHome}>Games</Button>}
        <Button size="sm" variant="flat" onPress={onLeaderboard}>🏆</Button>
      </div>
    </div>
  );
}

// ── tile ─────────────────────────────────────────────────────────────────────

function Tile({ letter, blank, size, highlight, dragging }: { letter: string; blank: boolean; size: number; highlight?: boolean; dragging?: boolean }) {
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: Math.max(3, size * 0.13),
        background: TILE_BG,
        color: TILE_INK,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        position: "relative",
        fontWeight: 800,
        fontSize: size * 0.5,
        lineHeight: 1,
        boxShadow: dragging
          ? "0 6px 14px rgba(0,0,0,.55)"
          : highlight
            ? "0 0 0 2px #f5a623, 0 1px 2px rgba(0,0,0,.5)"
            : "0 1px 2px rgba(0,0,0,.5), inset 0 -2px 0 rgba(0,0,0,.15)",
        userSelect: "none",
        touchAction: "none",
      }}
    >
      {letter === "_" ? "" : letter}
      <span style={{ position: "absolute", right: size * 0.08, bottom: size * 0.02, fontSize: size * 0.28, fontWeight: 700, opacity: blank ? 0.35 : 0.7 }}>
        {letterPoints(letter, blank)}
      </span>
    </div>
  );
}

// ── home ─────────────────────────────────────────────────────────────────────

function Home({ me, onOpen, onNew }: { me: string; onOpen: (id: string) => void; onNew: () => void }) {
  const [games, setGames] = useState<GameSummary[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    wf.games().then(setGames).catch((e) => setErr(String(e.message || e)));
  }, []);
  return (
    <div style={{ ...colFill, padding: 12 }}>
      <Button color="warning" fullWidth onPress={onNew} style={{ fontWeight: 700, flexShrink: 0 }}>＋ New Game</Button>
      <div style={{ flex: 1, minHeight: 0, overflowY: "auto", marginTop: 12, display: "flex", flexDirection: "column", gap: 8 }}>
        {err && <p style={{ color: "#f87171" }}>{err}</p>}
        {!games && !err && <div style={{ textAlign: "center", marginTop: 20 }}><Spinner /></div>}
        {games && games.length === 0 && <p style={{ color: "#9aa4b2", textAlign: "center", marginTop: 24 }}>No games yet — start one above.</p>}
        {games?.map((g) => {
          const yourTurn = g.status === "active" && g.current_player_id === me;
          return (
            <button key={g.id} onClick={() => onOpen(g.id)} style={cardBtn}>
              <div style={{ textAlign: "left" }}>
                <div style={{ fontWeight: 700 }}>{g.name}</div>
                <div style={{ fontSize: 13, color: "#9aa4b2", marginTop: 2 }}>{g.players.map((p) => `${p.name} ${p.score}`).join("  ·  ")}</div>
              </div>
              {g.status !== "active" ? (
                <Chip size="sm" variant="flat">{g.winner_id ? (g.players.find((p) => p.id === g.winner_id)?.name ?? "") + " won" : "Done"}</Chip>
              ) : yourTurn ? (
                <Chip size="sm" color="warning">Your turn</Chip>
              ) : (
                <Chip size="sm" variant="flat">{g.current_player_name}</Chip>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ── new game ─────────────────────────────────────────────────────────────────

function NewGame({ me, onCreated, onCancel }: { me: string; onCreated: (id: string) => void; onCancel: () => void }) {
  const [players, setPlayers] = useState<Player[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    wf.players().then((ps) => setPlayers(ps.filter((p) => p.id !== me))).catch((e) => setErr(String(e.message || e)));
  }, [me]);
  const toggle = (id: string) => {
    const n = new Set(picked);
    n.has(id) ? n.delete(id) : n.add(id);
    setPicked(n);
  };
  const create = async () => {
    setBusy(true);
    setErr(null);
    try {
      const g = await wf.create(name.trim() || "Family Game", Array.from(picked));
      onCreated(g.id);
    } catch (e: any) {
      setErr(String(e.message || e));
      setBusy(false);
    }
  };
  return (
    <div style={{ ...colFill, padding: 12 }}>
      <input value={name} onChange={(e: any) => setName(e.target.value)} placeholder="Game name (optional)" style={inputStyle} />
      <p style={{ color: "#9aa4b2", fontSize: 13, margin: "12px 0 6px", flexShrink: 0 }}>Who's playing?</p>
      <div style={{ flex: 1, minHeight: 0, overflowY: "auto", display: "flex", flexDirection: "column", gap: 8 }}>
        {!players && <Spinner />}
        {players?.map((p) => (
          <button key={p.id} onClick={() => toggle(p.id)} style={pickRow(picked.has(p.id))}>
            <Avatar name={p.name} size="sm" style={{ background: p.color }} />
            <span style={{ fontWeight: 600 }}>{p.name}</span>
            <span style={{ marginLeft: "auto", color: picked.has(p.id) ? "#f5a623" : "#586069" }}>{picked.has(p.id) ? "✓" : ""}</span>
          </button>
        ))}
      </div>
      {err && <p style={{ color: "#f87171", margin: "8px 0" }}>{err}</p>}
      <div style={{ display: "flex", gap: 8, marginTop: 12, flexShrink: 0 }}>
        <Button variant="flat" onPress={onCancel} fullWidth>Cancel</Button>
        <Button color="warning" fullWidth isDisabled={picked.size === 0 || busy} onPress={create}>{busy ? "Starting…" : `Start (${picked.size + 1})`}</Button>
      </div>
    </div>
  );
}

// ── game ─────────────────────────────────────────────────────────────────────

interface Staged extends Placement {
  rackIndex: number;
}
const keyOf = (r: number, c: number) => `${r},${c}`;

function GameScreen({ me, gameId }: { me: string; gameId: string }) {
  const [game, setGame] = useState<GameView | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [order, setOrder] = useState<number[]>([]);
  const [staged, setStaged] = useState<Staged[]>([]);
  const [blankAt, setBlankAt] = useState<{ row: number; col: number; rackIndex: number } | null>(null);
  const [swapSel, setSwapSel] = useState<Set<number> | null>(null);
  const [busy, setBusy] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);
  const [wide, setWide] = useState(false);
  const [boardPx, setBoardPx] = useState(0);

  const boardWrapRef = useRef<any>(null);
  const boardRef = useRef<any>(null);
  const dragData = useRef<{ tile: string; rackIndex: number; fromKey?: string } | null>(null);
  const [dragPos, setDragPos] = useState<{ x: number; y: number; tile: string; blank: boolean } | null>(null);
  const gameRef = useRef<GameView | null>(null);
  gameRef.current = game;
  const stagedRef = useRef<Staged[]>([]);
  stagedRef.current = staged;

  const refresh = useCallback(async (resetTurn = false) => {
    try {
      const g = await wf.game(gameId);
      setGame(g);
      if (resetTurn) {
        setOrder(g.your_rack.map((_, i) => i));
        setStaged([]);
        setSwapSel(null);
      }
    } catch (e: any) {
      setErr(String(e.message || e));
    }
  }, [gameId]);

  useEffect(() => { refresh(true); }, [refresh]);

  const myTurn = !!game && game.status === "active" && game.current_player_id === me;
  useEffect(() => {
    if (!game || game.finished || myTurn) return;
    const t = setInterval(() => refresh(true), 6000);
    return () => clearInterval(t);
  }, [game, myTurn, refresh]);

  useEffect(() => {
    if (game && order.length !== game.your_rack.length) {
      setOrder(game.your_rack.map((_, i) => i));
      setStaged([]);
    }
  }, [game, order.length]);

  // Fit the board to its container; choose row/column layout by aspect.
  useEffect(() => {
    const el = boardWrapRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(() => {
      const r = el.getBoundingClientRect();
      setBoardPx(Math.max(120, Math.floor(Math.min(r.width, r.height)) - 4));
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [game, wide]);

  useEffect(() => {
    const onR = () => setWide(window.innerWidth >= 780 && window.innerWidth > window.innerHeight);
    onR();
    window.addEventListener("resize", onR);
    return () => window.removeEventListener("resize", onR);
  }, []);

  const stagedAt = useMemo(() => {
    const m: Record<string, Staged> = {};
    for (const s of staged) m[keyOf(s.row, s.col)] = s;
    return m;
  }, [staged]);
  const usedRack = useMemo(() => new Set(staged.map((s) => s.rackIndex)), [staged]);

  // ── drag and drop ──
  const onMove = useCallback((e: any) => {
    setDragPos((p) => (p ? { ...p, x: e.clientX, y: e.clientY } : p));
  }, []);
  const onUp = useCallback((e: any) => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    const d = dragData.current;
    dragData.current = null;
    setDragPos(null);
    if (!d) return;
    const el = boardRef.current;
    const g = gameRef.current;
    if (!el || !g) return; // tile returns to rack
    const r = el.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    if (!inside) return; // dropped off board → back to rack
    const col = Math.min(14, Math.max(0, Math.floor((e.clientX - r.left) / (r.width / 15))));
    const row = Math.min(14, Math.max(0, Math.floor((e.clientY - r.top) / (r.height / 15))));
    if (d.fromKey === keyOf(row, col)) return; // dropped back where it started → recall
    if (g.board[row][col]) return; // committed tile there
    if (d.tile === "_") {
      setBlankAt({ row, col, rackIndex: d.rackIndex });
      return;
    }
    setStaged((prev) => {
      if (prev.some((x) => x.row === row && x.col === col)) return prev;
      return [...prev, { row, col, letter: d.tile, is_blank: false, rackIndex: d.rackIndex }];
    });
  }, [onMove]);

  const startDrag = (e: any, tile: string, rackIndex: number, fromKey?: string) => {
    e.preventDefault();
    if (fromKey) setStaged((s) => s.filter((x) => keyOf(x.row, x.col) !== fromKey));
    dragData.current = { tile, rackIndex, fromKey };
    setDragPos({ x: e.clientX, y: e.clientY, tile, blank: tile === "_" });
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  if (err) return <div style={{ ...colFill, alignItems: "center", justifyContent: "center", color: "#f87171" }}>{err}</div>;
  if (!game) return <div style={{ ...colFill, alignItems: "center", justifyContent: "center" }}><Spinner /></div>;

  const cellPx = boardPx / 15;
  const rackTile = Math.max(30, Math.min(54, Math.floor(boardPx / 8.5)));

  const chooseBlank = (letter: string) => {
    if (!blankAt) return;
    setStaged((s) => [...s, { row: blankAt.row, col: blankAt.col, letter, is_blank: true, rackIndex: blankAt.rackIndex }]);
    setBlankAt(null);
  };

  const doPlay = async () => {
    if (staged.length === 0) return;
    setBusy(true);
    try {
      const placements: Placement[] = staged.map(({ row, col, letter, is_blank }) => ({ row, col, letter, is_blank }));
      const res = await wf.play(gameId, placements);
      setGame(res.game);
      setStaged([]);
      setOrder(res.game.your_rack.map((_, i) => i));
      flashMsg(res.outcome ? `+${res.outcome.score} · ${res.outcome.words.map((w) => w.text).join(", ")}` : "Played");
    } catch (e: any) {
      flashMsg(`✗ ${String(e.message || e)}`);
    } finally {
      setBusy(false);
    }
  };
  const doSwap = async () => {
    if (!swapSel || swapSel.size === 0) return;
    setBusy(true);
    try {
      const tiles = Array.from(swapSel).map((i) => game.your_rack[i]);
      const res = await wf.swap(gameId, tiles);
      setGame(res.game);
      setSwapSel(null);
      setOrder(res.game.your_rack.map((_, i) => i));
      flashMsg("Swapped");
    } catch (e: any) {
      flashMsg(`✗ ${String(e.message || e)}`);
    } finally {
      setBusy(false);
    }
  };
  const act = async (fn: () => Promise<{ game: GameView }>, label: string) => {
    setBusy(true);
    try {
      const res = await fn();
      setGame(res.game);
      setStaged([]);
      flashMsg(label);
    } catch (e: any) {
      flashMsg(`✗ ${String(e.message || e)}`);
    } finally {
      setBusy(false);
    }
  };
  let flashTimer: any;
  function flashMsg(m: string) {
    setFlash(m);
    clearTimeout(flashTimer);
    flashTimer = setTimeout(() => setFlash(null), 3500);
  }

  const rackEl = (
    <Rack
      rack={game.your_rack}
      order={order}
      used={usedRack}
      size={rackTile}
      swapSel={swapSel}
      onPointerDownTile={(e, i) => {
        if (swapSel) {
          const n = new Set(swapSel);
          n.has(i) ? n.delete(i) : n.add(i);
          setSwapSel(n);
        } else {
          startDrag(e, game.your_rack[i], i);
        }
      }}
      onShuffle={() => setOrder((o) => [...o].sort(() => Math.random() - 0.5))}
    />
  );
  const controlsEl = myTurn && !game.finished && (
    <Controls
      busy={busy}
      staged={staged.length}
      swapSel={swapSel}
      onPlay={doPlay}
      onRecall={() => setStaged([])}
      onSwapToggle={() => setSwapSel(swapSel ? null : new Set())}
      onSwapConfirm={doSwap}
      onPass={() => { if (confirm("Pass your turn?")) act(() => wf.pass(gameId), "Passed"); }}
      onResign={() => { if (confirm("Resign this game?")) act(() => wf.resign(gameId), "Resigned"); }}
    />
  );
  const waitingEl = !myTurn && !game.finished && (
    <p style={{ textAlign: "center", color: "#9aa4b2", fontSize: 13, padding: "6px 0" }}>
      Waiting for {game.players.find((p) => p.id === game.current_player_id)?.name ?? "opponent"}…
    </p>
  );

  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: wide ? "row" : "column", gap: 8, padding: 8 }}>
      <div ref={boardWrapRef} style={{ flex: 1, minWidth: 0, minHeight: 0, display: "flex", alignItems: "center", justifyContent: "center", order: wide ? 0 : 1 }}>
        <Board boardPx={boardPx} cellPx={cellPx} board={game.board} stagedAt={stagedAt} boardRef={boardRef} onStagedDown={(e, r, c, t) => startDrag(e, t, stagedAt[keyOf(r, c)].rackIndex, keyOf(r, c))} />
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, flexShrink: 0, width: wide ? 300 : "100%", minHeight: 0, order: wide ? 1 : 0 }}>
        <Scores game={game} me={me} />
        {game.finished && (
          <div style={{ textAlign: "center", color: "#f5a623", fontWeight: 700, fontSize: 14 }}>
            {game.winner_id ? (game.players.find((p) => p.id === game.winner_id)?.name ?? "") + " wins!" : "Draw"}
          </div>
        )}
        {!game.finished && <div style={{ order: wide ? 3 : 0 }}>{rackEl}</div>}
        {controlsEl}
        {waitingEl}
        {wide && <div style={{ flex: 1, minHeight: 0, overflowY: "auto" }}><History gameId={gameId} moveCount={game.move_count} /></div>}
      </div>

      {flash && (
        <div style={{ position: "absolute", left: "50%", bottom: 14, transform: "translateX(-50%)", background: flash.startsWith("✗") ? "#7f1d1d" : "#065f46", color: "#fff", padding: "6px 14px", borderRadius: 999, fontWeight: 600, fontSize: 13, zIndex: 1500 }}>
          {flash}
        </div>
      )}
      {dragPos && (
        <div style={{ position: "fixed", left: dragPos.x, top: dragPos.y, transform: "translate(-50%,-55%)", pointerEvents: "none", zIndex: 2000, opacity: 0.95 }}>
          <Tile letter={dragPos.tile} blank={dragPos.blank} size={Math.max(rackTile, cellPx)} dragging />
        </div>
      )}
      {blankAt && <BlankPicker onPick={chooseBlank} onCancel={() => setBlankAt(null)} />}
    </div>
  );
}

function Scores({ game, me }: { game: GameView; me: string }) {
  return (
    <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
      {game.players.map((p) => {
        const active = p.id === game.current_player_id && !game.finished;
        return (
          <div key={p.id} style={{ flex: 1, padding: "6px 10px", borderRadius: 10, background: active ? "rgba(245,166,35,.12)" : "rgba(255,255,255,.03)", border: active ? "1px solid #f5a623" : "1px solid rgba(255,255,255,.06)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <span style={{ width: 9, height: 9, borderRadius: 9, background: p.color }} />
              <span style={{ fontSize: 12, fontWeight: 600 }}>{p.name}{p.id === me ? " (you)" : ""}{p.resigned ? " ⚐" : ""}</span>
            </div>
            <div style={{ fontSize: 20, fontWeight: 800 }}>{p.score}</div>
          </div>
        );
      })}
      <div style={{ alignSelf: "center", fontSize: 12, color: "#9aa4b2", paddingLeft: 4 }}>bag<br />{game.bag_remaining}</div>
    </div>
  );
}

function Board({ boardPx, cellPx, board, stagedAt, boardRef, onStagedDown }: any) {
  return (
    <div ref={boardRef} style={{ width: boardPx, height: boardPx, display: "grid", gridTemplateColumns: "repeat(15, 1fr)", gridTemplateRows: "repeat(15, 1fr)", gap: 1, background: BOARD_BG, padding: 2, borderRadius: 6, touchAction: "none" }}>
      {Array.from({ length: 15 }).map((_, r) =>
        Array.from({ length: 15 }).map((__, c) => {
          const committed = board[r][c];
          const st = stagedAt[`${r},${c}`];
          const prem = premiumAt(r, c);
          let bg = CELL_BG;
          let label = "";
          if (prem && prem !== "START" && PREMIUM_COLOR[prem]) {
            bg = PREMIUM_COLOR[prem];
            label = PREMIUM_LABEL[prem] ?? "";
          }
          return (
            <div key={`${r},${c}`} style={{ position: "relative", background: committed || st ? "transparent" : bg, borderRadius: 2, display: "flex", alignItems: "center", justifyContent: "center", fontSize: cellPx * 0.34, fontWeight: 700, color: "rgba(255,255,255,.8)" }}>
              {committed ? (
                <FillTile letter={committed.letter} blank={committed.blank} cellPx={cellPx} />
              ) : st ? (
                <div onPointerDown={(e: any) => onStagedDown(e, r, c, st.letter)} style={{ position: "absolute", inset: 0, touchAction: "none", cursor: "grab" }}>
                  <FillTile letter={st.letter} blank={st.is_blank} cellPx={cellPx} highlight />
                </div>
              ) : prem === "START" ? (
                <span style={{ color: "#7c3aed", fontSize: cellPx * 0.5 }}>★</span>
              ) : (
                <span style={{ opacity: 0.7 }}>{label}</span>
              )}
            </div>
          );
        }),
      )}
    </div>
  );
}

function FillTile({ letter, blank, cellPx, highlight }: { letter: string; blank: boolean; cellPx: number; highlight?: boolean }) {
  return (
    <div style={{ position: "absolute", inset: 0, borderRadius: 2, background: TILE_BG, color: TILE_INK, display: "flex", alignItems: "center", justifyContent: "center", fontWeight: 800, fontSize: cellPx * 0.52, lineHeight: 1, boxShadow: highlight ? "0 0 0 2px #f5a623" : "inset 0 -1px 0 rgba(0,0,0,.2)" }}>
      {letter === "_" ? "" : letter}
      <span style={{ position: "absolute", right: cellPx * 0.06, bottom: 0, fontSize: cellPx * 0.3, opacity: blank ? 0.4 : 0.7 }}>{letterPoints(letter, blank)}</span>
    </div>
  );
}

function Rack({ rack, order, used, size, swapSel, onPointerDownTile, onShuffle }: any) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0, padding: 8, background: "rgba(255,255,255,.04)", borderRadius: 10, minHeight: size + 16 }}>
      <div style={{ display: "flex", gap: 5, flex: 1, justifyContent: "center", flexWrap: "wrap" }}>
        {order.map((i: number) => {
          if (used.has(i)) return <div key={i} style={{ width: size, height: size, borderRadius: 6, background: "rgba(255,255,255,.04)" }} />;
          const sel = swapSel && swapSel.has(i);
          return (
            <div key={i} onPointerDown={(e: any) => onPointerDownTile(e, i)} style={{ cursor: "grab", touchAction: "none", filter: sel ? "saturate(.4) brightness(1.3)" : "none", transform: sel ? "translateY(-5px)" : "none" }}>
              <Tile letter={rack[i]} blank={rack[i] === "_"} size={size} highlight={!!sel} />
            </div>
          );
        })}
      </div>
      <Button isIconOnly size="sm" variant="light" onPress={onShuffle} title="Shuffle">⇄</Button>
    </div>
  );
}

function Controls({ busy, staged, swapSel, onPlay, onRecall, onSwapToggle, onSwapConfirm, onPass, onResign }: any) {
  if (swapSel) {
    return (
      <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
        <Button variant="flat" fullWidth onPress={onSwapToggle}>Cancel</Button>
        <Button color="warning" fullWidth isDisabled={swapSel.size === 0 || busy} onPress={onSwapConfirm}>Swap {swapSel.size || ""}</Button>
      </div>
    );
  }
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, flexShrink: 0 }}>
      <div style={{ display: "flex", gap: 8 }}>
        <Button color="warning" fullWidth isDisabled={staged === 0 || busy} onPress={onPlay} style={{ fontWeight: 700 }}>Play{staged ? ` (${staged})` : ""}</Button>
        <Button variant="flat" isDisabled={staged === 0} onPress={onRecall}>Recall</Button>
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <Button size="sm" variant="light" fullWidth onPress={onSwapToggle}>Swap</Button>
        <Button size="sm" variant="light" fullWidth onPress={onPass}>Pass</Button>
        <Button size="sm" variant="light" color="danger" fullWidth onPress={onResign}>Resign</Button>
      </div>
    </div>
  );
}

function BlankPicker({ onPick, onCancel }: { onPick: (l: string) => void; onCancel: () => void }) {
  const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");
  return (
    <div onClick={onCancel} style={overlay}>
      <div onClick={(e: any) => e.stopPropagation()} style={{ background: "#1b2430", padding: 16, borderRadius: 12, maxWidth: 320 }}>
        <p style={{ fontWeight: 700, marginBottom: 10, textAlign: "center" }}>Pick a letter for the blank</p>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(7, 1fr)", gap: 5 }}>
          {letters.map((l) => (<button key={l} onClick={() => onPick(l)} style={blankKey}>{l}</button>))}
        </div>
      </div>
    </div>
  );
}

function History({ gameId, moveCount }: { gameId: string; moveCount: number }) {
  const [moves, setMoves] = useState<MoveRecord[] | null>(null);
  useEffect(() => { wf.history(gameId).then(setMoves).catch(() => setMoves([])); }, [gameId, moveCount]);
  return (
    <div style={{ fontSize: 13 }}>
      <div style={{ color: "#9aa4b2", fontWeight: 600, margin: "4px 0" }}>History</div>
      {moves?.length === 0 && <p style={{ color: "#9aa4b2" }}>No moves yet.</p>}
      {moves?.slice().reverse().map((m) => (
        <div key={m.id} style={{ display: "flex", justifyContent: "space-between", padding: "3px 0", borderBottom: "1px solid rgba(255,255,255,.05)" }}>
          <span style={{ color: "#cdd5df" }}>{m.player_name} <span style={{ color: "#7d8590" }}>{m.kind === "play" ? m.words.map((w) => w.text).join(", ") : m.kind}</span></span>
          <span style={{ color: "#34d399", fontWeight: 600 }}>{m.kind === "play" ? `+${m.score}` : ""}</span>
        </div>
      ))}
    </div>
  );
}

function Leaderboard() {
  const [rows, setRows] = useState<LeaderRow[] | null>(null);
  useEffect(() => { wf.leaderboard().then(setRows).catch(() => setRows([])); }, []);
  return (
    <div style={{ ...colFill, padding: 12 }}>
      <h2 style={{ fontSize: 16, fontWeight: 700, marginBottom: 10, flexShrink: 0 }}>🏆 Standings</h2>
      <div style={{ flex: 1, minHeight: 0, overflowY: "auto", display: "flex", flexDirection: "column", gap: 8 }}>
        {!rows && <Spinner />}
        {rows?.length === 0 && <p style={{ color: "#9aa4b2" }}>No finished games yet.</p>}
        {rows?.map((r, i) => (
          <div key={r.player_id} style={{ display: "flex", alignItems: "center", gap: 12, padding: 10, borderRadius: 10, background: "rgba(255,255,255,.04)" }}>
            <span style={{ fontWeight: 800, color: "#7d8590", width: 18 }}>{i + 1}</span>
            <span style={{ width: 10, height: 10, borderRadius: 10, background: r.color }} />
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: 700 }}>{r.name}</div>
              <div style={{ fontSize: 12, color: "#9aa4b2" }}>{r.games_won}W / {r.games_played}G · {r.total_points} pts{r.best_word ? ` · best ${r.best_word} (${r.best_word_score})` : ""}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── styles ──
const inputStyle: any = { width: "100%", padding: "10px 12px", borderRadius: 8, background: "rgba(255,255,255,.05)", border: "1px solid rgba(255,255,255,.1)", color: "#e6edf3", outline: "none", flexShrink: 0, boxSizing: "border-box" };
const overlay: any = { position: "fixed", inset: 0, background: "rgba(0,0,0,.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 3000, padding: 16 };
const blankKey: any = { padding: "8px 0", borderRadius: 6, background: TILE_BG, color: TILE_INK, border: "none", fontWeight: 800, cursor: "pointer" };
const cardBtn: any = { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 12px", borderRadius: 10, background: "rgba(255,255,255,.04)", border: "1px solid rgba(255,255,255,.07)", cursor: "pointer", color: "#e6edf3" };
function pickRow(active: boolean): any {
  return { display: "flex", alignItems: "center", gap: 10, padding: "8px 12px", borderRadius: 10, background: active ? "rgba(245,166,35,.12)" : "rgba(255,255,255,.04)", border: active ? "1px solid #f5a623" : "1px solid rgba(255,255,255,.08)", cursor: "pointer", color: "#e6edf3" };
}
