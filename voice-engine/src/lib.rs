//! Pure TTS arbitration engine for harbour-opencode.
//!
//! This crate deliberately has **no dependencies at all** (no Qt, no tokio, no
//! filesystem). It models the decisions that used to be split between the QML
//! layer and the Rust worker — and therefore raced:
//!
//! * which texts are waiting to be synthesised (FIFO);
//! * whether the single synthesis pump task should run;
//! * whether playback may start *now* (never over the user's dictation, never
//!   more than one item at a time);
//! * how a "stop" invalidates work that is already in flight.
//!
//! The Rust worker owns one [`TtsEngine`] inside a `Mutex`; QML is a dumb
//! player that only executes `[[tts]]` / `[[tts-stop]]` lines and reports back
//! when a file finished playing. All arbitration lives here, so the state
//! machine can be unit-tested exhaustively without a device.

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

/// Answer playback mode: `Auto` — speak every answer, `Button` — only via the
/// 🔉 button on the bubble, `Off` — off. Single source of the protocol strings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TtsMode {
    Auto,
    Button,
    Off,
}

impl Default for TtsMode {
    fn default() -> Self {
        TtsMode::Button
    }
}

impl TtsMode {
    pub const ALL: [TtsMode; 3] = [TtsMode::Auto, TtsMode::Button, TtsMode::Off];

    pub fn as_str(self) -> &'static str {
        match self {
            TtsMode::Auto => "auto",
            TtsMode::Button => "button",
            TtsMode::Off => "off",
        }
    }

    pub fn parse(s: &str) -> Option<TtsMode> {
        match s {
            "auto" => Some(TtsMode::Auto),
            "button" => Some(TtsMode::Button),
            "off" => Some(TtsMode::Off),
            _ => None,
        }
    }
}

/// Whether the caller must spawn the synthesis pump task after [`TtsEngine::enqueue`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pump {
    /// The pump was not running — spawn it.
    Start,
    /// The pump is already alive and will pick the item up.
    AlreadyRunning,
}

/// What the pump should do next (result of [`TtsEngine::next_synth`]).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SynthStep {
    /// Synthesise this text; pass the token to [`TtsEngine::finish_synth`].
    Work(String, u64),
    /// The user is dictating — keep the pump alive and retry later.
    Wait,
    /// Nothing left to do; the pump flag has been cleared, the task may exit.
    Done,
}

/// A synthesised item waiting to be played (`body` is the bubble text it is
/// bound to, echoed back to QML so the stop/speak button follows it).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ready {
    pub path: String,
    pub body: String,
}

/// Single-owner TTS arbitration state.
#[derive(Default)]
pub struct TtsEngine {
    /// Texts waiting for synthesis, in arrival order.
    synth_queue: VecDeque<String>,
    /// Synthesised items waiting for playback, in arrival order.
    ready: VecDeque<Ready>,
    /// The pump task is alive (or about to be spawned).
    pump_running: bool,
    /// Path of the item currently playing (QML is expected to ack it).
    playing: Option<String>,
    /// Generation counter, bumped on every [`TtsEngine::stop`]. A synthesis
    /// that started under an older generation is discarded when it finishes.
    gen: u64,
}

impl TtsEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue `text` for synthesis. Returns whether the pump must be spawned.
    /// Empty/whitespace-only text is ignored entirely.
    pub fn enqueue(&mut self, text: impl Into<String>) -> Option<Pump> {
        let text = text.into();
        if text.trim().is_empty() {
            return None;
        }
        self.synth_queue.push_back(text);
        Some(if self.pump_running {
            Pump::AlreadyRunning
        } else {
            self.pump_running = true;
            Pump::Start
        })
    }

    /// The next text to synthesise. `dictating` postpones synthesis (the mic
    /// and the synthesiser must not fight over the CPU). On [`SynthStep::Done`]
    /// the pump flag is cleared atomically, so a concurrent `enqueue` that
    /// arrives afterwards correctly asks for a fresh pump.
    pub fn next_synth(&mut self, dictating: bool) -> SynthStep {
        if dictating {
            // Pump stays alive; drop stale work if a stop happened meanwhile.
            if self.synth_queue.is_empty() {
                self.pump_running = false;
                return SynthStep::Done;
            }
            return SynthStep::Wait;
        }
        match self.synth_queue.pop_front() {
            Some(text) => SynthStep::Work(text, self.gen),
            None => {
                self.pump_running = false;
                SynthStep::Done
            }
        }
    }

    /// Records a finished synthesis. Returns `false` when `gen` is stale (a
    /// stop happened while piper was running) — the caller must delete the WAV
    /// and not enqueue it. This is what prevents a cancelled answer from
    /// "resurrecting" after the fact.
    pub fn finish_synth(&mut self, gen: u64, path: impl Into<String>, body: impl Into<String>) -> bool {
        if gen != self.gen {
            return false;
        }
        let text = body.into();
        if text.trim().is_empty() {
            return false;
        }
        self.ready.push_back(Ready { path: path.into(), body: text });
        true
    }

    /// Pop the next item for playback if it is allowed right now: nothing may
    /// play while the user dictates, and only one item may play at a time.
    /// Marks the item as playing; the caller then expects a
    /// [`TtsEngine::playback_finished`] ack.
    pub fn next_play(&mut self, dictating: bool) -> Option<Ready> {
        if dictating || self.playing.is_some() {
            return None;
        }
        let item = self.ready.pop_front()?;
        self.playing = Some(item.path.clone());
        Some(item)
    }

    /// Confirm that the item with `path` finished playing. Returns `true` when
    /// it was indeed the current item (a duplicate/stale ack returns `false`
    /// and leaves the state untouched).
    pub fn playback_finished(&mut self, path: &str) -> bool {
        if self.playing.as_deref() == Some(path) {
            self.playing = None;
            true
        } else {
            false
        }
    }

    /// Stop everything: drop the synthesis queue and the ready queue, abort the
    /// current playback and invalidate any synthesis already in flight (its
    /// result is discarded by [`TtsEngine::finish_synth`] via the generation).
    pub fn stop(&mut self) {
        self.gen = self.gen.wrapping_add(1);
        self.synth_queue.clear();
        self.ready.clear();
        self.playing = None;
    }

    /// Whether a file is currently marked as playing.
    pub fn is_playing(&self) -> bool {
        self.playing.is_some()
    }

    /// Path of the item currently playing, if any.
    pub fn playing_path(&self) -> Option<&str> {
        self.playing.as_deref()
    }

    /// Number of synthesised items waiting for playback.
    pub fn ready_len(&self) -> usize {
        self.ready.len()
    }

    /// Number of texts waiting for synthesis.
    pub fn pending_synth_len(&self) -> usize {
        self.synth_queue.len()
    }

    /// Whether the pump task is considered alive.
    pub fn pump_running(&self) -> bool {
        self.pump_running
    }
}

/// Stable cache file name for a (model, spoken text) pair. Deterministic, so a
/// repeated answer for the same model reuses the WAV.
pub fn cache_file_name(model: &str, spoken: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    model.hash(&mut h);
    0u8.hash(&mut h);
    spoken.hash(&mut h);
    format!("{:016x}.wav", h.finish())
}

/// Strip markdown formatting before synthesis: headings, lists, block quotes,
/// code fences, inline links and the markup symbols (`**` `~` `#` `|` `>`) — so
/// the speech engine gets clean prose instead of reading the markup characters.
pub fn strip_markdown_for_tts(md: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in md.lines() {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || t.is_empty() {
            continue;
        }
        // Horizontal rules.
        if t.chars().all(|c| "-*_~".contains(c)) && t.chars().count() >= 3 {
            continue;
        }
        // Headings, block quotes and table separators: drop the leading marker.
        let rest = t
            .trim_start_matches(|c| c == '#' || c == '>' || c == '|')
            .trim_start();
        // Bullet lists: "- x", "* x", "+ x".
        let rest = rest
            .strip_prefix('-')
            .or_else(|| rest.strip_prefix('*'))
            .or_else(|| rest.strip_prefix('+'))
            .map(|s| s.trim_start())
            .unwrap_or(rest);
        // Numbered lists: "1. x", "12. x".
        let rest = {
            let bytes = rest.as_bytes();
            let mut i = 0;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > 0 && i < bytes.len() && bytes[i] == b'.' {
                rest[i + 1..].trim_start()
            } else {
                rest
            }
        };
        let cleaned = clean_inline_tts(rest);
        if !cleaned.is_empty() {
            out.push_str(&cleaned);
            out.push(' ');
        }
    }
    out
}

/// Inline pass: keep the link label and drop backticks plus the markup symbols.
/// Everything is read character by character, so no regex crate is needed.
fn clean_inline_tts(raw: &str) -> String {
    let mut src = raw;
    let mut out = String::with_capacity(raw.len());
    while !src.is_empty() {
        if let Some(i) = src.find('[') {
            out.push_str(&drop_markup(&src[..i]));
            match src[i..].find(']') {
                Some(j) => {
                    let label = &src[i + 1..i + j];
                    let after = &src[i + j + 1..];
                    // "[label](url)" → label; a lone "[" → label without adornment.
                    if let Some(after) = after.strip_prefix("(") {
                        match after.find(')') {
                            Some(k) => {
                                out.push_str(&clean_inline_tts(label));
                                out.push(' ');
                                src = &after[k + 1..];
                                continue;
                            }
                            None => {}
                        }
                    }
                    out.push_str(&clean_inline_tts(label));
                    out.push(' ');
                    src = after;
                }
                None => {
                    out.push_str(&drop_markup(&src[i..]));
                    break;
                }
            }
        } else {
            out.push_str(&drop_markup(src));
            break;
        }
    }
    out.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Remove the characters that mark bold/italic/strike/code/inline table cells.
fn drop_markup(s: &str) -> String {
    s.chars().filter(|c| !"`*_~|#>".contains(*c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(path: &str, body: &str) -> Ready {
        Ready { path: path.to_string(), body: body.to_string() }
    }

    // ---- TtsMode --------------------------------------------------------

    #[test]
    fn mode_roundtrip() {
        for m in TtsMode::ALL {
            assert_eq!(TtsMode::parse(m.as_str()), Some(m));
        }
    }

    #[test]
    fn mode_default_is_button() {
        assert_eq!(TtsMode::default(), TtsMode::Button);
        assert_eq!(TtsMode::default().as_str(), "button");
    }

    #[test]
    fn mode_parse_rejects_unknown() {
        assert_eq!(TtsMode::parse(""), None);
        assert_eq!(TtsMode::parse("Auto"), None);
        assert_eq!(TtsMode::parse("text"), None);
    }

    // ---- enqueue / pump -------------------------------------------------

    #[test]
    fn first_enqueue_starts_pump_second_does_not() {
        let mut e = TtsEngine::new();
        assert_eq!(e.enqueue("a"), Some(Pump::Start));
        assert_eq!(e.enqueue("b"), Some(Pump::AlreadyRunning));
        assert!(e.pump_running());
        assert_eq!(e.pending_synth_len(), 2);
    }

    #[test]
    fn empty_text_is_ignored() {
        let mut e = TtsEngine::new();
        assert_eq!(e.enqueue(""), None);
        assert_eq!(e.enqueue("   \n\t"), None);
        // Whitespace-only never starts a pump nor queues anything.
        assert!(!e.pump_running());
        assert_eq!(e.pending_synth_len(), 0);
        assert_eq!(e.enqueue("  hello  "), Some(Pump::Start));
        assert_eq!(e.pending_synth_len(), 1);
    }

    #[test]
    fn enqueue_restarts_pump_after_done() {
        let mut e = TtsEngine::new();
        assert_eq!(e.enqueue("a"), Some(Pump::Start));
        assert_eq!(e.next_synth(false), SynthStep::Work("a".into(), 0));
        assert_eq!(e.next_synth(false), SynthStep::Done);
        assert!(!e.pump_running());
        // A new item must ask for a fresh pump.
        assert_eq!(e.enqueue("b"), Some(Pump::Start));
        assert!(e.pump_running());
    }

    // ---- next_synth -----------------------------------------------------

    #[test]
    fn next_synth_drains_in_fifo_order() {
        let mut e = TtsEngine::new();
        e.enqueue("a");
        e.enqueue("b");
        e.enqueue("c");
        assert_eq!(e.next_synth(false), SynthStep::Work("a".into(), 0));
        assert_eq!(e.next_synth(false), SynthStep::Work("b".into(), 0));
        assert_eq!(e.next_synth(false), SynthStep::Work("c".into(), 0));
        assert_eq!(e.next_synth(false), SynthStep::Done);
    }

    #[test]
    fn next_synth_waits_while_dictating() {
        let mut e = TtsEngine::new();
        e.enqueue("a");
        assert_eq!(e.next_synth(true), SynthStep::Wait);
        assert!(e.pump_running());
        assert_eq!(e.pending_synth_len(), 1);
        assert_eq!(e.next_synth(false), SynthStep::Work("a".into(), 0));
    }

    #[test]
    fn next_synth_waits_then_done_after_stop_while_dictating() {
        let mut e = TtsEngine::new();
        e.enqueue("a");
        e.stop();
        // Even while dictating, a stop empties the queue → pump exits.
        assert_eq!(e.next_synth(true), SynthStep::Done);
        assert!(!e.pump_running());
    }

    // ---- finish_synth / generation -------------------------------------

    #[test]
    fn finish_synth_accepts_current_generation() {
        let mut e = TtsEngine::new();
        e.enqueue("a");
        let (_, gen) = match e.next_synth(false) {
            SynthStep::Work(t, g) => (t, g),
            other => panic!("{other:?}"),
        };
        assert!(e.finish_synth(gen, "/tmp/a.wav", "a"));
        assert_eq!(e.ready_len(), 1);
    }

    #[test]
    fn finish_synth_discards_stale_generation() {
        let mut e = TtsEngine::new();
        e.enqueue("a");
        let gen = match e.next_synth(false) {
            SynthStep::Work(_, g) => g,
            other => panic!("{other:?}"),
        };
        // The user stopped (or started dictating) while piper ran.
        e.stop();
        assert!(!e.finish_synth(gen, "/tmp/a.wav", "a"));
        assert_eq!(e.ready_len(), 0);
    }

    #[test]
    fn finish_synth_rejects_empty_body() {
        let mut e = TtsEngine::new();
        assert!(!e.finish_synth(0, "/tmp/a.wav", "   "));
        assert_eq!(e.ready_len(), 0);
    }

    // ---- playback gating -----------------------------------------------

    #[test]
    fn only_one_item_plays_at_a_time() {
        let mut e = TtsEngine::new();
        e.finish_synth(0, "/a.wav", "a");
        e.finish_synth(0, "/b.wav", "b");
        assert_eq!(e.next_play(false), Some(ready("/a.wav", "a")));
        // While "a" is playing nothing else may start.
        assert_eq!(e.next_play(false), None);
        assert!(e.is_playing());
        assert_eq!(e.playing_path(), Some("/a.wav"));
        assert_eq!(e.ready_len(), 1);
    }

    #[test]
    fn playback_is_deferred_while_dictating() {
        let mut e = TtsEngine::new();
        e.finish_synth(0, "/a.wav", "a");
        assert_eq!(e.next_play(true), None);
        assert!(!e.is_playing());
        assert_eq!(e.next_play(false), Some(ready("/a.wav", "a")));
    }

    #[test]
    fn ack_advances_to_next_item() {
        let mut e = TtsEngine::new();
        e.finish_synth(0, "/a.wav", "a");
        e.finish_synth(0, "/b.wav", "b");
        assert_eq!(e.next_play(false), Some(ready("/a.wav", "a")));
        assert!(e.playback_finished("/a.wav"));
        assert!(!e.is_playing());
        assert_eq!(e.next_play(false), Some(ready("/b.wav", "b")));
    }

    #[test]
    fn stale_ack_does_not_advance() {
        let mut e = TtsEngine::new();
        e.finish_synth(0, "/a.wav", "a");
        assert_eq!(e.next_play(false), Some(ready("/a.wav", "a")));
        // Wrong path (e.g. a stop-induced StoppedState) must be ignored.
        assert!(!e.playback_finished("/other.wav"));
        assert_eq!(e.playing_path(), Some("/a.wav"));
        // Duplicate ack of the same item is harmless the second time.
        assert!(e.playback_finished("/a.wav"));
        assert!(!e.playback_finished("/a.wav"));
    }

    // ---- stop -----------------------------------------------------------

    #[test]
    fn stop_clears_everything() {
        let mut e = TtsEngine::new();
        e.enqueue("a");
        e.finish_synth(0, "/a.wav", "a");
        let _ = e.next_play(false);
        e.stop();
        assert_eq!(e.pending_synth_len(), 0);
        assert_eq!(e.ready_len(), 0);
        assert!(!e.is_playing());
        assert_eq!(e.next_play(false), None);
    }

    #[test]
    fn stop_does_not_touch_an_enqueue_that_came_after() {
        let mut e = TtsEngine::new();
        e.enqueue("old");
        let old_gen = match e.next_synth(false) {
            SynthStep::Work(_, g) => g,
            other => panic!("{other:?}"),
        };
        e.stop();
        // A brand new request after the stop belongs to the new generation.
        // The pump may still be alive mid-cycle, so it joins the existing run.
        assert!(matches!(e.enqueue("new"), Some(Pump::Start | Pump::AlreadyRunning)));
        let new_gen = match e.next_synth(false) {
            SynthStep::Work(t, g) => {
                assert_eq!(t, "new");
                g
            }
            other => panic!("{other:?}"),
        };
        assert_ne!(old_gen, new_gen);
        assert!(!e.finish_synth(old_gen, "/old.wav", "old"));
        assert!(e.finish_synth(new_gen, "/new.wav", "new"));
        assert_eq!(e.ready_len(), 1);
    }

    // ---- end-to-end scenarios ------------------------------------------

    #[test]
    fn scenario_three_part_answer_plays_in_order() {
        let mut e = TtsEngine::new();
        // Worker streams three final parts.
        e.enqueue("one");
        e.enqueue("two");
        e.enqueue("three");
        // Pump synthesises all three ahead of playback.
        for (i, t) in ["one", "two", "three"].iter().enumerate() {
            let gen = match e.next_synth(false) {
                SynthStep::Work(t, g) => {
                    assert_eq!(t, *t);
                    g
                }
                other => panic!("{other:?}"),
            };
            assert!(e.finish_synth(gen, &format!("/{i}.wav"), *t));
        }
        assert_eq!(e.next_synth(false), SynthStep::Done);
        // Playback follows the text order, gated by acks.
        assert_eq!(e.next_play(false), Some(ready("/0.wav", "one")));
        assert!(e.playback_finished("/0.wav"));
        assert_eq!(e.next_play(false), Some(ready("/1.wav", "two")));
        assert!(e.playback_finished("/1.wav"));
        assert_eq!(e.next_play(false), Some(ready("/2.wav", "three")));
        assert!(e.playback_finished("/2.wav"));
        assert_eq!(e.next_play(false), None);
    }

    #[test]
    fn scenario_dictation_interrupts_playback_and_silences_queue() {
        let mut e = TtsEngine::new();
        e.finish_synth(0, "/a.wav", "a");
        e.finish_synth(0, "/b.wav", "b");
        assert_eq!(e.next_play(false), Some(ready("/a.wav", "a")));
        // User presses PTT mid-playback → stop.
        e.stop();
        assert!(!e.is_playing());
        assert_eq!(e.ready_len(), 0);
        // The stop-induced ack from the player is ignored.
        assert!(!e.playback_finished("/a.wav"));
        assert_eq!(e.next_play(false), None);
    }

    #[test]
    fn scenario_part_finalised_while_dictating_is_spoken_after() {
        let mut e = TtsEngine::new();
        // A part is finalised but the user is dictating: it may not play.
        e.finish_synth(0, "/a.wav", "a");
        assert_eq!(e.next_play(true), None);
        // Dictation ends → the pending item is played, not lost.
        assert_eq!(e.next_play(false), Some(ready("/a.wav", "a")));
    }

    #[test]
    fn scenario_synthesis_during_dictation_defers_then_resumes() {
        let mut e = TtsEngine::new();
        e.enqueue("a");
        // Pump wakes while dictating → waits, keeps the work queued.
        assert_eq!(e.next_synth(true), SynthStep::Wait);
        assert_eq!(e.pending_synth_len(), 1);
        assert!(e.pump_running());
        // Dictation ends → the same text is synthesised.
        assert_eq!(e.next_synth(false), SynthStep::Work("a".into(), 0));
    }

    // ---- markdown stripping --------------------------------------------

    #[test]
    fn strips_headings_lists_and_quotes() {
        let md = "# Title\n- bullet one\n> quoted\n1. numbered\n";
        assert_eq!(strip_markdown_for_tts(md), "Title bullet one quoted numbered ");
    }

    #[test]
    fn strips_code_fences_entirely() {
        let md = "before\n```rust\nlet x = 1;\n```\nafter";
        assert_eq!(strip_markdown_for_tts(md), "before after ");
    }

    #[test]
    fn keeps_link_label_drops_url() {
        let md = "see [the docs](https://example.com/a) now";
        assert_eq!(strip_markdown_for_tts(md), "see the docs now ");
    }

    #[test]
    fn drops_inline_markup_symbols() {
        let md = "**bold** and _italic_ and `code` and ~~gone~~";
        assert_eq!(strip_markdown_for_tts(md), "bold and italic and code and gone ");
    }

    #[test]
    fn drops_horizontal_rules() {
        let md = "text\n---\nmore";
        assert_eq!(strip_markdown_for_tts(md), "text more ");
    }

    #[test]
    fn empty_and_code_only_input() {
        assert_eq!(strip_markdown_for_tts(""), "");
        assert_eq!(strip_markdown_for_tts("```\nonly code\n```"), "");
    }

    // ---- cache key ------------------------------------------------------

    #[test]
    fn cache_name_is_deterministic_and_case_sensitive() {
        assert_eq!(cache_file_name("m", "hello"), cache_file_name("m", "hello"));
        assert_ne!(cache_file_name("m", "hello"), cache_file_name("m", "Hello"));
        assert_ne!(cache_file_name("m1", "hello"), cache_file_name("m2", "hello"));
        assert!(cache_file_name("m", "hello").ends_with(".wav"));
    }
}
