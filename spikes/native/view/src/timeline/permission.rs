//! A waiting permission request as Paseo's permission card draws it
//! (`agent-stream/view.tsx`, `PermissionRequestCard`; a plan request as
//! its `PlanCard`): what the card's title says, the plan or the command it
//! shows, its answers (their words from [`answer_words`], which is
//! primary, the mark before each), and GOAT's "1/3" when several wait
//! (`ComposerPendingApprovalPanel`).
//!
//! [Decision Log]
//! - Purpose: the owner on the old card ("허가 여부 ui 디자인 개떡같네
//!   paseo나 goat 보고 배워"): a red box, a bold sentence wrapped round the
//!   whole command, four filled pills. The card is Paseo's, value for
//!   value; these are its rules, apart from drawing.
//! - The references: Paseo's title is the request's title, else its name,
//!   else "Permission Required" ("Plan" for a plan); its detail is the
//!   tool's (a shell call's `$ command`, an edit's diff); its answers are
//!   the agent's, the one marked primary in the foreground and the rest
//!   muted, a check before an allow and a cross before a reject. GOAT
//!   counts "1/N" when more than one waits.
//! - Constraint: Claude Code's adapter titles a shell call with the
//!   command itself, so Paseo's title and its box say the same long line
//!   twice; Codex sends its command as `["bash", "-lc", "…"]`; Claude Code
//!   leaving plan mode puts the plan's Markdown under `plan` in the tool's
//!   input; ACP marks no answer primary.
//! - Choice: Paseo's rules, and: a title that is the command falls back to
//!   "Permission Required", so the command is read once, in its box;
//!   Codex's shell wrapper is read as its script; the primary answer is the
//!   one Approve would take (the first "once" allow, else the first allow,
//!   the page's and GOAT's rule).

use vessel_proto::{PermissionOption, PermissionOptionKind, ToolKind};

use super::approval::{AnswerWords, allows, answer_words, answers_of};
use super::entry::{Permission, ToolView};

/// What the card's title says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardTitle<'a> {
    /// The agent's own title for the request.
    Agent(&'a str),
    /// A plan to approve: "Plan".
    Plan,
    /// The agent gave none (or only the command): "Permission Required".
    Required,
}

/// The mark before an answer's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerMark {
    /// A check: the answer allows.
    Allow,
    /// A cross: the answer rejects.
    Reject,
}

/// One answer on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardAnswer<'a> {
    /// The agent's option.
    pub option: &'a PermissionOption,
    /// What its button says.
    pub words: AnswerWords<'a>,
    /// Its mark.
    pub mark: AnswerMark,
    /// Whether its words and mark are in the foreground (the rest muted).
    pub primary: bool,
}

/// A waiting request, as the card draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionCard<'a> {
    /// Its title.
    pub title: CardTitle<'a>,
    /// A plan's Markdown: the card is Paseo's plan card.
    pub plan: Option<String>,
    /// A shell call's command, for its box.
    pub command: Option<String>,
    /// The answers, in the agent's order.
    pub answers: Vec<CardAnswer<'a>>,
    /// "1/N" when N > 1 wait.
    pub count: Option<(usize, usize)>,
}

/// The card for `permission`, its tool call when seen, with `waiting`
/// requests waiting in all. `say` puts an answer's kind in the person's
/// words (for [`answer_words`]).
#[must_use]
pub fn permission_card<'a>(
    permission: &'a Permission,
    tool: Option<&ToolView>,
    waiting: usize,
    say: impl Fn(PermissionOptionKind) -> &'static str,
) -> PermissionCard<'a> {
    let input = tool
        .and_then(|tool| tool.input.as_ref())
        .and_then(|input| serde_json::from_str::<serde_json::Value>(input.as_text()).ok());
    let plan = input
        .as_ref()
        .and_then(|input| input.get("plan"))
        .and_then(serde_json::Value::as_str)
        .filter(|plan| !plan.trim().is_empty())
        .map(str::to_owned);
    let shell = tool.is_none_or(|tool| tool.kind == ToolKind::Execute);
    let command = input
        .as_ref()
        .and_then(|input| input.get("command"))
        .filter(|_| shell)
        .and_then(command_text);
    let own = permission
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty());
    let title = if plan.is_some() {
        CardTitle::Plan
    } else {
        match own {
            Some(title) if command.as_deref().map(str::trim) != Some(title) => {
                CardTitle::Agent(title)
            }
            Some(_) | None => CardTitle::Required,
        }
    };
    let options = &permission.request.options;
    let primary = answers_of(options).approve;
    let answers = options
        .iter()
        .zip(answer_words(options, say))
        .map(|(option, words)| CardAnswer {
            option,
            words,
            mark: if allows(option.kind) {
                AnswerMark::Allow
            } else {
                AnswerMark::Reject
            },
            primary: primary.is_some_and(|primary| std::ptr::eq(primary, option)),
        })
        .collect();
    PermissionCard {
        title,
        plan,
        command,
        answers,
        count: (waiting > 1).then_some((1, waiting)),
    }
}

/// A command as the agent gave it: a line, or the words of a call
/// (`["bash", "-lc", "script"]` reads as its script).
fn command_text(command: &serde_json::Value) -> Option<String> {
    if let Some(line) = command.as_str() {
        return Some(line.trim_end().to_owned()).filter(|line| !line.is_empty());
    }
    let words: Vec<&str> = command
        .as_array()?
        .iter()
        .map(serde_json::Value::as_str)
        .collect::<Option<_>>()?;
    match words.as_slice() {
        [_, flag, script] if matches!(*flag, "-c" | "-lc") => Some((*script).to_owned()),
        [] => None,
        words => Some(words.join(" ")),
    }
}
