//! An entry's words: its meta line, and the line said for it aloud or
//! copied (the page's `timeline-item.tsx`, `data/timeline-text.ts`,
//! `data/approval.ts`, `data/tool-runs.ts`, `data/models.ts`).

use vessel_proto::{
    ModelApi, ModelCall, ModelFailure, ModelStop, ProfileName, QuestionOutcome, SessionStatus,
    StopReason, ToolCallStatus, TurnUsage,
};

use super::ConversationWords;
use super::numbers::{grouped, short_tokens};
use super::plural;
use crate::Lang;
use crate::timeline::{Entry, EntryKind, FailureKind, PlanActivity, plan_progress, turn_failure};
use crate::timeline::{RunSummary, Tally};
use crate::timeline::{Settled, settled};

impl ConversationWords {
    /// "Agent ready".
    #[must_use]
    pub const fn agent_ready(&self) -> &'static str {
        self.either("Agent ready", "에이전트 준비됨")
    }

    /// A prompt's corner: its turn, or that the harness replayed it.
    #[must_use]
    pub fn prompt_corner(&self, replayed: bool, turn: u32) -> String {
        match (replayed, self.lang) {
            (true, _) => self.either("replayed", "다시 불러옴").to_owned(),
            (false, Lang::En) => format!("turn {turn}"),
            (false, Lang::Ko) => format!("{turn}번째 턴"),
        }
    }

    /// A steered message's corner.
    #[must_use]
    pub const fn steered(&self) -> &'static str {
        self.either("steered", "끼어든 메시지")
    }

    /// A thought's fold.
    #[must_use]
    pub const fn thought(&self) -> &'static str {
        self.either("Thought", "생각")
    }

    /// A tool call with no title.
    #[must_use]
    pub const fn tool_call(&self) -> &'static str {
        self.either("Tool call", "도구 호출")
    }

    /// A permission whose tool call was not seen.
    #[must_use]
    pub const fn unseen_tool_call(&self) -> &'static str {
        self.either("A tool call", "도구 호출")
    }

    /// A tool call's status, as its badge says it.
    #[must_use]
    pub const fn tool_status(&self, status: ToolCallStatus) -> &'static str {
        match status {
            ToolCallStatus::Pending => self.either("pending", "대기"),
            ToolCallStatus::InProgress => self.either("in progress", "진행 중"),
            ToolCallStatus::Completed => self.either("completed", "완료"),
            ToolCallStatus::Failed => self.either("failed", "실패"),
        }
    }

    /// Why a turn stopped.
    #[must_use]
    pub const fn stop_reason(&self, reason: StopReason) -> &'static str {
        match reason {
            StopReason::EndTurn => self.either("end turn", "정상 종료"),
            StopReason::MaxTokens => self.either("max tokens", "토큰 한도"),
            StopReason::MaxTurnRequests => self.either("max turn requests", "요청 횟수 한도"),
            StopReason::Refusal => self.either("refusal", "거절"),
            StopReason::Cancelled => self.either("cancelled", "취소됨"),
        }
    }

    /// A session's state, as its status line says it.
    #[must_use]
    pub const fn state(&self, status: &SessionStatus) -> &'static str {
        match status {
            SessionStatus::Starting => self.either("starting", "시작 중"),
            SessionStatus::Running => self.either("running", "실행 중"),
            SessionStatus::Exited { .. } => self.either("exited", "종료됨"),
            SessionStatus::Failed { .. } => self.either("failed", "실패"),
            SessionStatus::Interrupted => self.either("interrupted", "중단됨"),
        }
    }

    /// A session's status as its badge says it: `exited 0`.
    #[must_use]
    pub fn status_word(&self, status: &SessionStatus) -> String {
        match (status, self.lang) {
            (SessionStatus::Exited { code: Some(code) }, Lang::En) => format!("exited {code}"),
            (SessionStatus::Exited { code: Some(code) }, Lang::Ko) => format!("종료됨 ({code})"),
            (
                SessionStatus::Starting
                | SessionStatus::Running
                | SessionStatus::Exited { code: None, .. }
                | SessionStatus::Failed { .. }
                | SessionStatus::Interrupted,
                Lang::En | Lang::Ko,
            ) => self.state(status).to_owned(),
        }
    }

    /// "Plan · 1 of 2 done".
    #[must_use]
    pub fn plan_line(&self, done: usize, total: usize) -> String {
        match self.lang {
            Lang::En => format!("Plan · {done} of {total} done"),
            Lang::Ko => format!("계획 · {total}개 중 {done}개 완료"),
        }
    }

    /// A plan row's words in the timeline (Paseo's `message.todo.activity`):
    /// "Created 4 tasks", or what happened to the step it names ("Added",
    /// "Started", "Completed"; the step's own words follow, apart).
    #[must_use]
    pub fn plan_activity(&self, activity: &PlanActivity) -> String {
        match (activity, self.lang) {
            (PlanActivity::Created { count }, Lang::En) => format!(
                "Created {}",
                plural(u64::try_from(*count).unwrap_or(u64::MAX), "task", "tasks")
            ),
            (PlanActivity::Created { count }, Lang::Ko) => format!("작업 {count}개 생성"),
            (PlanActivity::Added(_), Lang::En) => "Added".to_owned(),
            (PlanActivity::Added(_), Lang::Ko) => "추가됨".to_owned(),
            (PlanActivity::Started(_), Lang::En) => "Started".to_owned(),
            (PlanActivity::Started(_), Lang::Ko) => "시작됨".to_owned(),
            (PlanActivity::Completed(_), Lang::En) => "Completed".to_owned(),
            (PlanActivity::Completed(_), Lang::Ko) => "완료됨".to_owned(),
        }
    }

    /// An opened plan row with no steps (Paseo's `message.todo.empty`).
    #[must_use]
    pub const fn plan_empty(&self) -> &'static str {
        match self.lang {
            Lang::En => "No tasks yet.",
            Lang::Ko => "아직 작업이 없습니다.",
        }
    }

    /// A turn's tokens: "30 tokens · 20 in · 10 out".
    #[must_use]
    pub fn turn_tokens(&self, usage: &TurnUsage) -> String {
        let mut parts = match self.lang {
            Lang::En => vec![
                format!("{} tokens", grouped(usage.total.get())),
                format!("{} in", grouped(usage.input.get())),
                format!("{} out", grouped(usage.output.get())),
            ],
            Lang::Ko => vec![
                format!("토큰 {}", grouped(usage.total.get())),
                format!("입력 {}", grouped(usage.input.get())),
                format!("출력 {}", grouped(usage.output.get())),
            ],
        };
        if let Some(cached) = usage.cached_read.filter(|cached| cached.get() > 0) {
            parts.push(match self.lang {
                Lang::En => format!("{} cached", grouped(cached.get())),
                Lang::Ko => format!("캐시 {}", grouped(cached.get())),
            });
        }
        parts.join(" · ")
    }

    /// "Turn ended · end turn", with its tokens when known.
    #[must_use]
    pub fn turn_ended(&self, reason: StopReason, usage: Option<&TurnUsage>) -> String {
        let head = format!(
            "{} · {}",
            self.either("Turn ended", "턴 종료"),
            self.stop_reason(reason)
        );
        match usage {
            None => head,
            Some(usage) => format!("{head} · {}", self.turn_tokens(usage)),
        }
    }

    /// A finished turn's footer: how long the turn worked (Paseo's
    /// `message.turnFooter.workedFor`), `duration` as Paseo writes it
    /// (14s, 2m 5s, 1h 3m).
    #[must_use]
    pub fn worked_for(&self, duration: &str) -> String {
        match self.lang {
            Lang::En => format!("Worked for {duration}"),
            Lang::Ko => format!("작업 시간 {duration}"),
        }
    }

    /// The timeline's line for a permission request: how it ended, then
    /// what it was about.
    #[must_use]
    pub fn settled_line(&self, what: &str, state: &Settled) -> String {
        match (state, self.lang) {
            (Settled::Pending, Lang::En) => format!("Waiting on permission · {what}"),
            (Settled::Pending, Lang::Ko) => format!("권한 대기 중 · {what}"),
            (Settled::Allowed(option), Lang::En) => format!("Allowed · {what} · {option}"),
            (Settled::Allowed(option), Lang::Ko) => format!("허용함 · {what} · {option}"),
            (Settled::Declined(option), Lang::En) => format!("Declined · {what} · {option}"),
            (Settled::Declined(option), Lang::Ko) => format!("거부함 · {what} · {option}"),
            (Settled::Cancelled, Lang::En) => format!("Permission cancelled · {what}"),
            (Settled::Cancelled, Lang::Ko) => format!("권한 요청 취소됨 · {what}"),
        }
    }

    /// How a question was settled, as its line says.
    #[must_use]
    pub const fn question_line(&self, outcome: &QuestionOutcome) -> &'static str {
        match outcome {
            QuestionOutcome::Answered { .. } => self.either("Answered", "답함"),
            QuestionOutcome::Declined => self.either("Skipped", "건너뜀"),
            QuestionOutcome::Cancelled => self.either("Question closed", "질문 닫힘"),
        }
    }

    /// "Profile changed from A to B"; no profile is the default.
    #[must_use]
    pub fn profile_line(&self, from: Option<&ProfileName>, to: Option<&ProfileName>) -> String {
        let name = |profile: Option<&ProfileName>| {
            profile.map_or_else(
                || self.either("default", "기본").to_owned(),
                ToString::to_string,
            )
        };
        match self.lang {
            Lang::En => format!("Profile changed from {} to {}", name(from), name(to)),
            Lang::Ko => format!("프로필 변경: {} → {}", name(from), name(to)),
        }
    }

    /// An API's name, as `vessel models` prints it (not translated).
    #[must_use]
    pub const fn api_name(&self, api: ModelApi) -> &'static str {
        match api {
            ModelApi::AnthropicMessages => "anthropic-messages",
            ModelApi::OpenAiResponses => "openai-responses",
            ModelApi::OpenAiChatCompletions => "openai-chat-completions",
        }
    }

    /// How a model call ended.
    #[must_use]
    pub const fn model_stop(&self, stop: ModelStop) -> &'static str {
        match stop {
            ModelStop::EndTurn => self.either("end turn", "정상 종료"),
            ModelStop::ToolUse => self.either("tool use", "도구 사용"),
            ModelStop::MaxTokens => self.either("max tokens", "토큰 한도"),
            ModelStop::StopSequence => self.either("stop sequence", "중단 시퀀스"),
            ModelStop::Refusal => self.either("refused", "거절"),
            ModelStop::Pause => self.either("paused", "일시 정지"),
            ModelStop::Error => self.either("error", "오류"),
        }
    }

    /// Why a model call failed.
    #[must_use]
    pub const fn model_failure(&self, failure: ModelFailure) -> &'static str {
        match failure {
            ModelFailure::RateLimited => self.either("rate limited", "요청 한도 초과"),
            ModelFailure::Overloaded => self.either("overloaded", "과부하"),
            ModelFailure::UpstreamError => self.either("upstream error", "상위 서비스 오류"),
            ModelFailure::RefusedRequest => self.either("refused request", "요청 거부"),
            ModelFailure::ContextTooLong => self.either("context too long", "컨텍스트 초과"),
            ModelFailure::Timeout => self.either("timeout", "시간 초과"),
            ModelFailure::Truncated => self.either("truncated", "잘림"),
            ModelFailure::Unauthorized => self.either("unauthorized", "인증 실패"),
        }
    }

    /// The parts of one model call's line: tokens, how it ended, how long.
    #[must_use]
    pub fn call_parts(&self, call: &ModelCall) -> Vec<String> {
        let usage = &call.usage;
        let cached = usage
            .cache_read
            .get()
            .saturating_add(usage.cache_write.get());
        let input = if cached > 0 {
            match self.lang {
                Lang::En => format!(
                    "{} in ({} cached)",
                    short_tokens(usage.total_input().get()),
                    short_tokens(cached)
                ),
                Lang::Ko => format!(
                    "입력 {} (캐시 {})",
                    short_tokens(usage.total_input().get()),
                    short_tokens(cached)
                ),
            }
        } else {
            match self.lang {
                Lang::En => format!("{} in", short_tokens(usage.input.get())),
                Lang::Ko => format!("입력 {}", short_tokens(usage.input.get())),
            }
        };
        let output = match self.lang {
            Lang::En => format!("{} out", short_tokens(usage.output.get())),
            Lang::Ko => format!("출력 {}", short_tokens(usage.output.get())),
        };
        let ended = match call.failure {
            None => self.model_stop(call.stop).to_owned(),
            Some(failure) => match self.lang {
                Lang::En => format!("failed: {}", self.model_failure(failure)),
                Lang::Ko => format!("실패: {}", self.model_failure(failure)),
            },
        };
        vec![input, output, ended, self.duration(call.duration_ms.get())]
    }

    /// A model call's whole line: "Model call · route · model · …".
    #[must_use]
    pub fn model_line(&self, call: &ModelCall) -> String {
        format!(
            "{} · {} · {} · {}",
            self.either("Model call", "모델 호출"),
            call.route,
            call.model,
            self.call_parts(call).join(" · ")
        )
    }

    /// The row for a run of the agent's own log lines (its adapter's
    /// error output): "Agent log · 10 lines".
    #[must_use]
    pub fn agent_log(&self, lines: usize) -> String {
        let lines = u64::try_from(lines).unwrap_or(u64::MAX);
        match self.lang {
            Lang::En => format!("Agent log · {}", plural(lines, "line", "lines")),
            Lang::Ko => format!("에이전트 로그 · {lines}줄"),
        }
    }

    /// The sentence for a run of tool calls: "Edited 2 files and ran 3
    /// commands"; "Thought" for a run of thoughts alone.
    #[must_use]
    pub fn run_summary(&self, summary: &RunSummary) -> String {
        let parts: Vec<String> = summary
            .parts()
            .into_iter()
            .map(|(tally, count)| self.tally(tally, u64::from(count)))
            .collect();
        if parts.is_empty() {
            return if summary.thoughts > 0 {
                self.thought().to_owned()
            } else {
                String::new()
            };
        }
        match self.lang {
            Lang::En => {
                let text = match parts.split_last() {
                    Some((last, [])) => last.clone(),
                    Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
                    None => String::new(),
                };
                let mut characters = text.chars();
                characters.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(characters).collect()
                })
            }
            Lang::Ko => parts.join(", "),
        }
    }

    fn tally(self, tally: Tally, count: u64) -> String {
        match (self.lang, tally) {
            (Lang::En, Tally::Edited) => format!("edited {}", plural(count, "file", "files")),
            (Lang::En, Tally::Ran) => {
                format!("ran {}", plural(count, "command", "commands"))
            }
            (Lang::En, Tally::Read) => format!("read {}", plural(count, "file", "files")),
            (Lang::En, Tally::Searched) => {
                format!("searched {}", plural(count, "time", "times"))
            }
            (Lang::En, Tally::Other) => {
                format!("used {}", plural(count, "other tool", "other tools"))
            }
            (Lang::Ko, Tally::Edited) => format!("파일 {count}개 편집"),
            (Lang::Ko, Tally::Ran) => format!("명령 {count}개 실행"),
            (Lang::Ko, Tally::Read) => format!("파일 {count}개 읽음"),
            (Lang::Ko, Tally::Searched) => format!("검색 {count}번"),
            (Lang::Ko, Tally::Other) => format!("다른 도구 {count}개 사용"),
        }
    }

    /// `entry` as one line of text: what its row shows, without its
    /// markup. Said aloud when it arrives, and what a copy of it holds.
    #[must_use]
    pub fn spoken(&self, entry: &Entry, catalog: &crate::localization::Translator) -> String {
        match &entry.kind {
            EntryKind::Ready { .. } => self.agent_ready().to_owned(),
            EntryKind::Prompt(prompt) => {
                format!("{}: {}", self.either("Prompt", "프롬프트"), prompt.text)
            }
            EntryKind::Steer { text, .. } => {
                format!("{}: {text}", self.either("Steered", "끼어든 메시지"))
            }
            EntryKind::Question { question, outcome } => match outcome {
                None => format!(
                    "{}: {}",
                    self.either("The agent asks", "에이전트의 질문"),
                    question.message
                ),
                Some(outcome) => self.question_line(outcome).to_owned(),
            },
            EntryKind::Message { text } => text.clone(),
            EntryKind::Thought { .. } => self.thought().to_owned(),
            EntryKind::Tool(tool) => {
                let title = if tool.title.is_empty() {
                    self.tool_call()
                } else {
                    &tool.title
                };
                format!("{title} · {}", self.tool_status(tool.status))
            }
            EntryKind::Plan { entries, .. } => {
                let (done, total) = plan_progress(entries);
                self.plan_line(done, total)
            }
            EntryKind::Permission(permission) => self.settled_line(
                permission
                    .title
                    .as_deref()
                    .unwrap_or(self.unseen_tool_call()),
                &settled(&permission.request.options, permission.outcome.as_ref()),
            ),
            EntryKind::TurnEnded { reason, usage, .. } => self.turn_ended(*reason, usage.as_ref()),
            // One sentence, without the JSON-RPC wrapper or the agent's
            // data; a usage limit's sentence alone (`timeline::failure`).
            EntryKind::TurnFailed { message } => {
                let failure = turn_failure(message);
                match failure.kind {
                    FailureKind::Limit => failure.sentence,
                    FailureKind::Other => format!(
                        "{} · {}",
                        self.either("Turn failed", "턴 실패"),
                        failure.sentence
                    ),
                }
            }
            EntryKind::Log { line } => line.clone(),
            EntryKind::Status { status } => match self.lang {
                Lang::En => format!("Session {}", self.state(status)),
                Lang::Ko => format!("세션 {}", self.state(status)),
            },
            EntryKind::Parking(change) => self.parking_line(change),
            // The source's title is the screen's to find (the fleet's).
            EntryKind::Forked { .. } => catalog.fork().from_other().to_owned(),
            EntryKind::Profile { from, to } => self.profile_line(from.as_ref(), to.as_ref()),
            EntryKind::Model(call) => format!(
                "{} · {} · {}",
                self.either("Model call", "모델 호출"),
                call.route,
                call.model
            ),
        }
    }
}
