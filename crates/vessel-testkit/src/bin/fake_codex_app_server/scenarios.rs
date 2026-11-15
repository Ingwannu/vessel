//! Deterministic scenarios; every provider process here is a private test fake.
use super::{
    Arc, AtomicBool, Exit, Server, Streaming, Value, Waiting, command, first_text, in_progress,
    json,
};
impl Server {
    pub(super) fn turn(&mut self, id: &Value, params: &Value) -> Result<(), Exit> {
        self.turns = self.turns.saturating_add(1);
        let turn = format!("turn-{}", self.turns);
        let prompt = first_text(params);
        self.last_turn = params.clone();
        if prompt.starts_with("badstart") {
            // A turn without its status: the answer does not read, and
            // nothing more is said of the turn.
            return self.send(&json!({"id": id, "result": {"turn": {"id": turn}}}));
        }
        self.send(&json!({"id": id, "result": {"turn": in_progress(&turn)}}))?;
        self.notify(
            "turn/started",
            &json!({"threadId": self.thread(), "turn": in_progress(&turn)}),
        )?;
        let scenario = prompt.split_whitespace().next().unwrap_or("").to_owned();
        match scenario.as_str() {
            "think" => {
                self.notify(
                    "item/reasoning/summaryTextDelta",
                    &json!({"threadId": self.thread(), "turnId": turn, "itemId": "think-1",
                        "summaryIndex": 0, "delta": "thinking it over"}),
                )?;
                self.plain(&turn, &prompt)
            }
            "quiet" => {
                self.message(&turn, "msg-quiet", "said all at once", false)?;
                self.completed(&turn, "completed", None)
            }
            "async-message" | "async-empty" => {
                let questions = if scenario == "async-empty" {
                    json!([])
                } else {
                    Value::Null
                };
                self.notify("item/completed", &json!({"threadId":self.thread(),"turnId":turn,"item":{"type":"agentMessage","id":"ordinary-async","delivery":"async","text":"ordinary asynchronous body","questions":questions}}))?;
                self.completed(&turn, "completed", None)
            }
            "tool" => {
                let started = command("cmd-1", "inProgress", None);
                self.notify(
                    "item/started",
                    &json!({"threadId": self.thread(), "turnId": turn, "startedAtMs": 1,
                        "item": started}),
                )?;
                let done = command("cmd-1", "completed", Some("hi\n"));
                self.notify(
                    "item/completed",
                    &json!({"threadId": self.thread(), "turnId": turn, "completedAtMs": 2,
                        "item": done}),
                )?;
                self.plain(&turn, &prompt)
            }
            "plan" => {
                self.notify(
                    "turn/plan/updated",
                    &json!({"threadId": self.thread(), "turnId": turn, "plan": [
                        {"step": "read", "status": "completed"},
                        {"step": "write", "status": "inProgress"}
                    ]}),
                )?;
                self.plain(&turn, &prompt)
            }
            "approve" | "resolved" | "edit" | "permit" | "ask" | "secret" | "form" | "policy" => {
                self.asking(&scenario, &turn)
            }
            "async-question" | "async-wait" | "async-duplicate" | "async-noack" => {
                self.async_question(&turn, &scenario)
            }
            "wait" => {
                self.waiting = Some(Waiting::Interrupt { turn });
                Ok(())
            }
            "fail" => self.completed(&turn, "failed", Some("fake failure")),
            "garbled" => self.notify(
                "turn/completed",
                &json!({"threadId": self.thread(), "turn": {"id": turn, "items": []}}),
            ),
            "other" => {
                self.completed("turn-of-another-client", "completed", None)?;
                self.plain(&turn, &prompt)
            }
            "stream" => {
                let stop = Arc::new(AtomicBool::new(false));
                let streaming = Streaming {
                    out: Arc::clone(&self.out),
                    thread: self.thread(),
                    turn: turn.clone(),
                    stop: Arc::clone(&stop),
                };
                self.waiting = Some(Waiting::Streaming { stop });
                drop(std::thread::spawn(move || streaming.run()));
                Ok(())
            }
            "exit" => Err(Exit::Now),
            _ => self.plain(&turn, &prompt),
        }
    }

    fn async_question(&mut self, turn: &str, scenario: &str) -> Result<(), Exit> {
        let item = json!({"type":"agentMessage","id":"async-offer-1","delivery":"async","text":"Choose a route", "questions":[{"title":"Route","options":["left","right"]}]});
        self.notify(
            "item/completed",
            &json!({"threadId":self.thread(),"turnId":turn,"item":item}),
        )?;
        if scenario == "async-duplicate" {
            self.notify(
                "item/completed",
                &json!({"threadId":self.thread(),"turnId":turn,"item":item}),
            )?;
        }
        if matches!(scenario, "async-wait" | "async-noack") {
            self.waiting = Some(Waiting::Interrupt {
                turn: turn.to_owned(),
            });
            Ok(())
        } else {
            self.completed(turn, "completed", None)
        }
    }
}
