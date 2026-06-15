/* @jsxRuntime classic */
// Words With Fam — embedded leo UI. Uses the shared React/HeroUI globals; tiles
// are placed by tapping (rack tile → board cell), which works well on phones.

const React = window.__LEO_SHARED__.React;
const { useState, useEffect, useMemo, useCallback, useRef } = React;
const HeroUI = window.__LEO_SHARED__.HeroUI;
const { Button, Card, CardBody, Chip, Spinner, Avatar } = HeroUI;

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
import type {
  GameView,
  GameSummary,
  Player,
  MoveRecord,
  LeaderRow,
  Placement,
} from "./lib";

type Screen =
  | { name: "home" }
  | { name: "new" }
  | { name: "game"; id: string }
  | { name: "leaderboard" };

// ─────────────────────────────────────────────────────────────────────────────

export default function WordsFam() {
  const me = wf.me();
  const [screen, setScreen] = useState<Screen>({ name: "home" });

  if (!me) {
    return (
      <Centered>
        <p style={{ color: "#9aa4b2" }}>Sign in to leo to play Words With Fam.</p>
      </Centered>
    );
  }

  return (
    <div style={{ maxWidth: 720, margin: "0 auto", padding: "16px 12px 48px" }}>
      <Header
        onHome={() => setScreen({ name: "home" })}
        onLeaderboard={() => setScreen({ name: "leaderboard" })}
      />
      {screen.name === "home" && (
        <Home
          me={me}
          onOpen={(id) => setScreen({ name: "game", id })}
          onNew={() => setScreen({ name: "new" })}
        />
      )}
      {screen.name === "new" && (
        <NewGame
          me={me}
          onCreated={(id) => setScreen({ name: "game", id })}
          onCancel={() => setScreen({ name: "home" })}
        />
      )}
      {screen.name === "game" && (
        <GameScreen me={me} gameId={screen.id} onBack={() => setScreen({ name: "home" })} />
      )}
      {screen.name === "leaderboard" && <Leaderboard />}
    </div>
  );
}

// ── shared bits ──────────────────────────────────────────────────────────────

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ display: "flex", justifyContent: "center", alignItems: "center", minHeight: 240 }}>
      {children}
    </div>
  );
}

function Header({ onHome, onLeaderboard }: { onHome: () => void; onLeaderboard: () => void }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        marginBottom: 16,
      }}
    >
      <button
        onClick={onHome}
        style={{ background: "none", border: "none", cursor: "pointer", textAlign: "left" }}
      >
        <span style={{ fontSize: 22, fontWeight: 800, color: "#e6edf3", letterSpacing: -0.5 }}>
          Words With <span style={{ color: "#f5a623" }}>Fam</span>
        </span>
      </button>
      <Button size="sm" variant="flat" onPress={onLeaderboard}>
        🏆 Standings
      </Button>
    </div>
  );
}

function Tile({ letter, blank, size = 30, highlight = false }: { letter: string; blank: boolean; size?: number; highlight?: boolean }) {
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: 5,
        background: TILE_BG,
        color: TILE_INK,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        position: "relative",
        fontWeight: 800,
        fontSize: size * 0.5,
        boxShadow: highlight
          ? "0 0 0 2px #f5a623, 0 1px 2px rgba(0,0,0,.5)"
          : "0 1px 2px rgba(0,0,0,.5), inset 0 -2px 0 rgba(0,0,0,.15)",
        userSelect: "none",
      }}
    >
      {letter === "_" ? "" : letter}
      <span style={{ position: "absolute", right: 2, bottom: 0, fontSize: size * 0.28, fontWeight: 700, opacity: blank ? 0.35 : 0.7 }}>
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
    <div>
      <Button color="warning" fullWidth onPress={onNew} style={{ fontWeight: 700, marginBottom: 16 }}>
        ＋ New Game
      </Button>
      {err && <p style={{ color: "#f87171" }}>{err}</p>}
      {!games && !err && <Centered><Spinner /></Centered>}
      {games && games.length === 0 && (
        <p style={{ color: "#9aa4b2", textAlign: "center", marginTop: 24 }}>
          No games yet — start one above.
        </p>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {games?.map((g) => {
          const yourTurn = g.status === "active" && g.current_player_id === me;
          return (
            <Card key={g.id} isPressable onPress={() => onOpen(g.id)} shadow="sm">
              <CardBody style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <div style={{ fontWeight: 700, color: "#e6edf3" }}>{g.name}</div>
                  <div style={{ fontSize: 13, color: "#9aa4b2", marginTop: 2 }}>
                    {g.players
                      .map((p) => `${p.name} ${p.score}`)
                      .join("  ·  ")}
                  </div>
                </div>
                {g.status !== "active" ? (
                  <Chip size="sm" variant="flat" color="default">
                    {g.winner_id ? (g.players.find((p) => p.id === g.winner_id)?.name ?? "Done") + " won" : "Done"}
                  </Chip>
                ) : yourTurn ? (
                  <Chip size="sm" color="warning" variant="solid">Your turn</Chip>
                ) : (
                  <Chip size="sm" variant="flat">{g.current_player_name}'s turn</Chip>
                )}
              </CardBody>
            </Card>
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
    const next = new Set(picked);
    next.has(id) ? next.delete(id) : next.add(id);
    setPicked(next);
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
    <div>
      <h2 style={{ fontSize: 18, fontWeight: 700, color: "#e6edf3", marginBottom: 12 }}>New game</h2>
      <input
        value={name}
        onChange={(e: any) => setName(e.target.value)}
        placeholder="Game name (optional)"
        style={inputStyle}
      />
      <p style={{ color: "#9aa4b2", fontSize: 13, margin: "16px 0 8px" }}>Who's playing?</p>
      {!players && <Spinner />}
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {players?.map((p) => (
          <button key={p.id} onClick={() => toggle(p.id)} style={pickRowStyle(picked.has(p.id))}>
            <Avatar name={p.name} size="sm" style={{ background: p.color }} />
            <span style={{ color: "#e6edf3", fontWeight: 600 }}>{p.name}</span>
            <span style={{ marginLeft: "auto", color: picked.has(p.id) ? "#f5a623" : "#586069" }}>
              {picked.has(p.id) ? "✓" : ""}
            </span>
          </button>
        ))}
      </div>
      {err && <p style={{ color: "#f87171", marginTop: 12 }}>{err}</p>}
      <div style={{ display: "flex", gap: 8, marginTop: 20 }}>
        <Button variant="flat" onPress={onCancel} fullWidth>Cancel</Button>
        <Button color="warning" fullWidth isDisabled={picked.size === 0 || busy} onPress={create}>
          {busy ? "Starting…" : `Start (${picked.size + 1} players)`}
        </Button>
      </div>
    </div>
  );
}

// ── game screen ──────────────────────────────────────────────────────────────

interface Staged extends Placement {
  rackIndex: number;
}

function GameScreen({ me, gameId, onBack }: { me: string; gameId: string; onBack: () => void }) {
  const [game, setGame] = useState<GameView | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [order, setOrder] = useState<number[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [staged, setStaged] = useState<Staged[]>([]);
  const [blankAt, setBlankAt] = useState<{ row: number; col: number; rackIndex: number } | null>(null);
  const [swapSel, setSwapSel] = useState<Set<number> | null>(null);
  const [busy, setBusy] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);

  const refresh = useCallback(
    async (resetTurn = false) => {
      try {
        const g = await wf.game(gameId);
        setGame(g);
        if (resetTurn) {
          setOrder(g.your_rack.map((_, i) => i));
          setStaged([]);
          setSelected(null);
          setSwapSel(null);
        }
      } catch (e: any) {
        setErr(String(e.message || e));
      }
    },
    [gameId],
  );

  useEffect(() => {
    refresh(true);
  }, [refresh]);

  // Poll while it's not our turn and the game is live.
  const myTurn = !!game && game.status === "active" && game.current_player_id === me;
  useEffect(() => {
    if (!game || game.finished || myTurn) return;
    const t = setInterval(() => refresh(true), 6000);
    return () => clearInterval(t);
  }, [game, myTurn, refresh]);

  // Keep rack order in sync if the rack size changes (after our move/refresh).
  useEffect(() => {
    if (game && order.length !== game.your_rack.length) {
      setOrder(game.your_rack.map((_, i) => i));
      setStaged([]);
      setSelected(null);
    }
  }, [game, order.length]);

  const stagedAt = useMemo(() => {
    const m: Record<string, Staged> = {};
    for (const s of staged) m[`${s.row},${s.col}`] = s;
    return m;
  }, [staged]);

  const usedRack = useMemo(() => new Set(staged.map((s) => s.rackIndex)), [staged]);

  if (err) return <ErrorBox msg={err} onBack={onBack} />;
  if (!game) return <Centered><Spinner /></Centered>;

  const onCell = (r: number, c: number) => {
    if (game.board[r][c]) return; // occupied by a committed tile
    const key = `${r},${c}`;
    if (stagedAt[key]) {
      // recall a staged tile
      setStaged((s) => s.filter((x) => !(x.row === r && x.col === c)));
      return;
    }
    if (selected == null) return;
    const tile = game.your_rack[selected];
    if (tile === "_") {
      setBlankAt({ row: r, col: c, rackIndex: selected });
      return;
    }
    setStaged((s) => [...s, { row: r, col: c, letter: tile, is_blank: false, rackIndex: selected }]);
    setSelected(null);
  };

  const chooseBlank = (letter: string) => {
    if (!blankAt) return;
    setStaged((s) => [...s, { row: blankAt.row, col: blankAt.col, letter, is_blank: true, rackIndex: blankAt.rackIndex }]);
    setBlankAt(null);
    setSelected(null);
  };

  const doPlay = async () => {
    if (staged.length === 0) return;
    setBusy(true);
    setErr(null);
    try {
      const placements: Placement[] = staged.map(({ row, col, letter, is_blank }) => ({ row, col, letter, is_blank }));
      const res = await wf.play(gameId, placements);
      setGame(res.game);
      setStaged([]);
      setSelected(null);
      setOrder(res.game.your_rack.map((_, i) => i));
      if (res.outcome) setFlash(`+${res.outcome.score} · ${res.outcome.words.map((w) => w.text).join(", ")}`);
    } catch (e: any) {
      setFlash(`✗ ${String(e.message || e)}`);
    } finally {
      setBusy(false);
    }
  };

  const recallAll = () => {
    setStaged([]);
    setSelected(null);
  };
  const shuffle = () => {
    setOrder((o) => [...o].sort(() => Math.random() - 0.5));
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
      setFlash("Swapped tiles");
    } catch (e: any) {
      setFlash(`✗ ${String(e.message || e)}`);
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
      setSelected(null);
      setFlash(label);
    } catch (e: any) {
      setFlash(`✗ ${String(e.message || e)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
        <Button size="sm" variant="light" onPress={onBack}>← Games</Button>
        <span style={{ fontWeight: 700, color: "#e6edf3" }}>{game.name}</span>
        <Chip size="sm" variant="flat" style={{ marginLeft: "auto" }}>Bag {game.bag_remaining}</Chip>
      </div>

      <ScorePanel game={game} me={me} />

      {game.finished && (
        <div style={{ textAlign: "center", margin: "10px 0", color: "#f5a623", fontWeight: 700 }}>
          Game over — {game.winner_id ? (game.players.find((p) => p.id === game.winner_id)?.name ?? "") + " wins!" : "draw"}
        </div>
      )}

      <Board board={game.board} stagedAt={stagedAt} onCell={onCell} />

      {flash && (
        <div style={{ textAlign: "center", margin: "8px 0", color: flash.startsWith("✗") ? "#f87171" : "#34d399", fontWeight: 600 }}>
          {flash}
        </div>
      )}

      {!game.finished && (
        <Rack
          rack={game.your_rack}
          order={order}
          used={usedRack}
          selected={selected}
          swapSel={swapSel}
          onTile={(i) => {
            if (swapSel) {
              const n = new Set(swapSel);
              n.has(i) ? n.delete(i) : n.add(i);
              setSwapSel(n);
            } else {
              setSelected(selected === i ? null : i);
            }
          }}
        />
      )}

      {!game.finished && myTurn && (
        <Controls
          busy={busy}
          staged={staged.length}
          swapSel={swapSel}
          onPlay={doPlay}
          onRecall={recallAll}
          onShuffle={shuffle}
          onSwapToggle={() => setSwapSel(swapSel ? null : new Set())}
          onSwapConfirm={doSwap}
          onPass={() => { if (confirm("Pass your turn?")) act(() => wf.pass(gameId), "Passed"); }}
          onResign={() => { if (confirm("Resign this game?")) act(() => wf.resign(gameId), "Resigned"); }}
        />
      )}
      {!game.finished && !myTurn && (
        <p style={{ textAlign: "center", color: "#9aa4b2", marginTop: 14 }}>
          Waiting for {game.players.find((p) => p.id === game.current_player_id)?.name ?? "opponent"}…
        </p>
      )}

      <History gameId={gameId} moveCount={game.move_count} />

      {blankAt && <BlankPicker onPick={chooseBlank} onCancel={() => setBlankAt(null)} />}
    </div>
  );
}

function ScorePanel({ game, me }: { game: GameView; me: string }) {
  return (
    <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
      {game.players.map((p) => {
        const active = p.id === game.current_player_id && !game.finished;
        return (
          <div
            key={p.id}
            style={{
              flex: 1,
              padding: "8px 10px",
              borderRadius: 10,
              background: active ? "rgba(245,166,35,.12)" : "rgba(255,255,255,.03)",
              border: active ? "1px solid #f5a623" : "1px solid rgba(255,255,255,.06)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <span style={{ width: 9, height: 9, borderRadius: 9, background: p.color, display: "inline-block" }} />
              <span style={{ fontSize: 13, fontWeight: 600, color: "#e6edf3" }}>
                {p.name}{p.id === me ? " (you)" : ""}{p.resigned ? " ⚐" : ""}
              </span>
            </div>
            <div style={{ fontSize: 22, fontWeight: 800, color: "#e6edf3" }}>{p.score}</div>
          </div>
        );
      })}
    </div>
  );
}

function Board({ board, stagedAt, onCell }: { board: any[][]; stagedAt: Record<string, any>; onCell: (r: number, c: number) => void }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(15, 1fr)",
        gap: 2,
        background: BOARD_BG,
        padding: 4,
        borderRadius: 8,
        width: "100%",
        aspectRatio: "1 / 1",
      }}
    >
      {Array.from({ length: 15 }).map((_, r) =>
        Array.from({ length: 15 }).map((__, c) => {
          const committed = board[r][c];
          const staged = stagedAt[`${r},${c}`];
          const prem = premiumAt(r, c);
          let bg = CELL_BG;
          let label = "";
          if (prem && prem !== "START" && PREMIUM_COLOR[prem]) {
            bg = PREMIUM_COLOR[prem];
            label = PREMIUM_LABEL[prem] ?? "";
          } else if (prem === "START") {
            bg = "transparent";
          }
          return (
            <div
              key={`${r},${c}`}
              onClick={() => onCell(r, c)}
              style={{
                position: "relative",
                aspectRatio: "1 / 1",
                background: committed || staged ? "transparent" : bg,
                borderRadius: 3,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                cursor: committed ? "default" : "pointer",
                fontSize: "1.6vw",
                fontWeight: 700,
                color: "rgba(255,255,255,.85)",
              }}
            >
              {committed ? (
                <FillTile letter={committed.letter} blank={committed.blank} />
              ) : staged ? (
                <FillTile letter={staged.letter} blank={staged.is_blank} highlight />
              ) : prem === "START" ? (
                <span style={{ color: "#7c3aed", fontSize: "2vw" }}>★</span>
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

// A board-filling tile that scales with the cell.
function FillTile({ letter, blank, highlight = false }: { letter: string; blank: boolean; highlight?: boolean }) {
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        borderRadius: 3,
        background: TILE_BG,
        color: TILE_INK,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontWeight: 800,
        fontSize: "2.1vw",
        boxShadow: highlight ? "0 0 0 2px #f5a623" : "inset 0 -1px 0 rgba(0,0,0,.2)",
      }}
    >
      {letter === "_" ? "" : letter}
      <span style={{ position: "absolute", right: 1, bottom: 0, fontSize: "1.1vw", opacity: blank ? 0.4 : 0.7 }}>
        {letterPoints(letter, blank)}
      </span>
    </div>
  );
}

function Rack({ rack, order, used, selected, swapSel, onTile }: { rack: string[]; order: number[]; used: Set<number>; selected: number | null; swapSel: Set<number> | null; onTile: (i: number) => void }) {
  return (
    <div
      style={{
        display: "flex",
        gap: 6,
        justifyContent: "center",
        margin: "14px 0",
        padding: 8,
        background: "rgba(255,255,255,.04)",
        borderRadius: 10,
        minHeight: 52,
      }}
    >
      {order.map((i) => {
        if (used.has(i)) return <div key={i} style={{ width: 38, height: 38, borderRadius: 6, background: "rgba(255,255,255,.04)" }} />;
        const sel = selected === i || (swapSel && swapSel.has(i));
        return (
          <button
            key={i}
            onClick={() => onTile(i)}
            style={{
              border: "none",
              padding: 0,
              cursor: "pointer",
              transform: sel ? "translateY(-6px)" : "none",
              transition: "transform .1s",
              filter: swapSel && swapSel.has(i) ? "saturate(.4) brightness(1.2)" : "none",
            }}
          >
            <Tile letter={rack[i]} blank={rack[i] === "_"} size={38} highlight={!!sel} />
          </button>
        );
      })}
    </div>
  );
}

function Controls({ busy, staged, swapSel, onPlay, onRecall, onShuffle, onSwapToggle, onSwapConfirm, onPass, onResign }: any) {
  if (swapSel) {
    return (
      <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
        <Button variant="flat" fullWidth onPress={onSwapToggle}>Cancel</Button>
        <Button color="warning" fullWidth isDisabled={swapSel.size === 0 || busy} onPress={onSwapConfirm}>
          Swap {swapSel.size || ""}
        </Button>
      </div>
    );
  }
  return (
    <div>
      <div style={{ display: "flex", gap: 8 }}>
        <Button color="warning" fullWidth isDisabled={staged === 0 || busy} onPress={onPlay} style={{ fontWeight: 700 }}>
          Play{staged ? ` (${staged})` : ""}
        </Button>
        <Button variant="flat" isDisabled={staged === 0} onPress={onRecall}>Recall</Button>
        <Button variant="flat" onPress={onShuffle}>Shuffle</Button>
      </div>
      <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
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
    <div onClick={onCancel} style={overlayStyle}>
      <div onClick={(e: any) => e.stopPropagation()} style={{ background: "#1b2430", padding: 16, borderRadius: 12, maxWidth: 320 }}>
        <p style={{ color: "#e6edf3", fontWeight: 700, marginBottom: 10, textAlign: "center" }}>Pick a letter for the blank</p>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(7, 1fr)", gap: 5 }}>
          {letters.map((l) => (
            <button key={l} onClick={() => onPick(l)} style={blankKeyStyle}>{l}</button>
          ))}
        </div>
      </div>
    </div>
  );
}

function History({ gameId, moveCount }: { gameId: string; moveCount: number }) {
  const [open, setOpen] = useState(false);
  const [moves, setMoves] = useState<MoveRecord[] | null>(null);
  useEffect(() => {
    if (open) wf.history(gameId).then(setMoves).catch(() => setMoves([]));
  }, [open, gameId, moveCount]);
  return (
    <div style={{ marginTop: 20 }}>
      <Button size="sm" variant="light" onPress={() => setOpen(!open)}>
        {open ? "▾" : "▸"} Move history
      </Button>
      {open && (
        <div style={{ marginTop: 8, fontSize: 13 }}>
          {!moves && <Spinner size="sm" />}
          {moves?.length === 0 && <p style={{ color: "#9aa4b2" }}>No moves yet.</p>}
          {moves?.slice().reverse().map((m) => (
            <div key={m.id} style={{ display: "flex", justifyContent: "space-between", padding: "4px 0", borderBottom: "1px solid rgba(255,255,255,.05)" }}>
              <span style={{ color: "#cdd5df" }}>
                {m.player_name}{" "}
                <span style={{ color: "#7d8590" }}>
                  {m.kind === "play" ? m.words.map((w) => w.text).join(", ") : m.kind}
                </span>
              </span>
              <span style={{ color: "#34d399", fontWeight: 600 }}>{m.kind === "play" ? `+${m.score}` : ""}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── leaderboard ──────────────────────────────────────────────────────────────

function Leaderboard() {
  const [rows, setRows] = useState<LeaderRow[] | null>(null);
  useEffect(() => {
    wf.leaderboard().then(setRows).catch(() => setRows([]));
  }, []);
  return (
    <div>
      <h2 style={{ fontSize: 18, fontWeight: 700, color: "#e6edf3", marginBottom: 12 }}>🏆 Standings</h2>
      {!rows && <Spinner />}
      {rows?.length === 0 && <p style={{ color: "#9aa4b2" }}>No finished games yet.</p>}
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {rows?.map((r, i) => (
          <Card key={r.player_id} shadow="sm">
            <CardBody style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
              <span style={{ fontWeight: 800, color: "#7d8590", width: 20 }}>{i + 1}</span>
              <span style={{ width: 10, height: 10, borderRadius: 10, background: r.color }} />
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 700, color: "#e6edf3" }}>{r.name}</div>
                <div style={{ fontSize: 12, color: "#9aa4b2" }}>
                  {r.games_won}W / {r.games_played}G · {r.total_points} pts
                  {r.best_word ? ` · best ${r.best_word} (${r.best_word_score})` : ""}
                </div>
              </div>
            </CardBody>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ── misc ─────────────────────────────────────────────────────────────────────

function ErrorBox({ msg, onBack }: { msg: string; onBack: () => void }) {
  return (
    <Centered>
      <div style={{ textAlign: "center" }}>
        <p style={{ color: "#f87171", marginBottom: 10 }}>{msg}</p>
        <Button size="sm" variant="flat" onPress={onBack}>← Back</Button>
      </div>
    </Centered>
  );
}

const inputStyle: any = {
  width: "100%",
  padding: "10px 12px",
  borderRadius: 8,
  background: "rgba(255,255,255,.05)",
  border: "1px solid rgba(255,255,255,.1)",
  color: "#e6edf3",
  outline: "none",
};
const overlayStyle: any = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,.6)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  zIndex: 1000,
  padding: 16,
};
const blankKeyStyle: any = {
  padding: "8px 0",
  borderRadius: 6,
  background: TILE_BG,
  color: TILE_INK,
  border: "none",
  fontWeight: 800,
  cursor: "pointer",
};
function pickRowStyle(active: boolean): any {
  return {
    display: "flex",
    alignItems: "center",
    gap: 10,
    padding: "8px 12px",
    borderRadius: 10,
    background: active ? "rgba(245,166,35,.12)" : "rgba(255,255,255,.04)",
    border: active ? "1px solid #f5a623" : "1px solid rgba(255,255,255,.08)",
    cursor: "pointer",
  };
}
