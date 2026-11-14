## A deliberately small complete source for the partial-language proof.
# Native language name; short.
meta-name = English
# General setting title; short.
meta-language = Language
# Welcome sentence. $name is the person's display name.
meta-welcome = Welcome, { $name }.
# Whole count sentence. $count remains numeric for the locale's plural rule.
meta-sessions = { NUMBER($count) ->
    [one] one session
   *[other] many sessions
    }
# Whole local-time sentence. $date carries a clock reading and zone label.
meta-time = Paired at { DATETIME($date) }.
