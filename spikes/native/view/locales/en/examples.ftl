## Contract examples, exercised by the localization tests; not screen copy.
# Welcome sentence. $name is the person's display name.
examples-welcome = Welcome, { $name }.
# A whole count sentence. $count is a number, never preformatted text.
examples-sessions = { NUMBER($count) ->
    [one] { $count } live session
   *[other] { $count } live sessions
    }
# A whole state sentence. $state is one of the selector's choices.
examples-state = { $state ->
    [connecting] Connecting to your machine.
   *[ready] Your machine is ready.
    }
# Object sentence. $name is a machine name; Korean chooses its particle.
examples-remove = Remove { $name }.
# Relative time. $count is numeric; $unit is the unit before formatting.
examples-ago = { $unit ->
    [minute] { NUMBER($count) ->
        [one] { $count } minute ago
       *[other] { $count } minutes ago
        }
   *[hour] { NUMBER($count) ->
        [one] { $count } hour ago
       *[other] { $count } hours ago
        }
    }

# Grouped count sentence. $count is numeric; grouping follows the language.
examples-grouped = { NUMBER($count) } sessions in total.
# Calendar date sentence. $date is a valid date, without a time zone.
examples-date = Paired on { DATETIME($date) }.
# Local time sentence. $date carries a valid clock reading and the zone label.
examples-time = Paired at { DATETIME($date) }.
# Local timestamp sentence. $date carries Gregorian date, clock and zone label.
examples-timestamp = Paired on { DATETIME($date) }.
# Future relative time. $count is numeric, with the language's plural rules.
examples-in-minutes = { NUMBER($count) ->
    [one] in { $count } minute
   *[other] in { $count } minutes
    }
