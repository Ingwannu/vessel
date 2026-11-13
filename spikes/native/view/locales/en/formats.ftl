## Formatting contract. These are data, not caller-selected language branches.
# Number grouping separator. A quoted empty string disables visible separators.
formats-group-separator = ,
# Decimal separator; must differ from the grouping separator.
formats-decimal-separator = .
# Exactly ten digit characters, in the order zero through nine.
formats-digits = 0123456789
# Digits in the rightmost group, from 1 to 9.
formats-group-primary = 3
# Digits in each earlier group; use 2 for a 3,2 grouping system.
formats-group-secondary = 3
# Clock convention for this language: h12 or h24.
formats-hour-cycle = h12
# Month name. $month is a number from 1 to 12.
formats-month = { $month ->
    [1] Jan
    [2] Feb
    [3] Mar
    [4] Apr
    [5] May
    [6] Jun
    [7] Jul
    [8] Aug
    [9] Sep
    [10] Oct
    [11] Nov
   *[12] Dec
    }
# Weekday name. $weekday is Monday=1 through Sunday=7.
formats-weekday = { $weekday ->
    [1] Mon
    [2] Tue
    [3] Wed
    [4] Thu
    [5] Fri
    [6] Sat
   *[7] Sun
    }
# Whole Gregorian date. Numeric $year/$day; names use $month/$weekday.
formats-date = { formats-weekday }, { formats-month } { NUMBER($day, useGrouping: "false") }, { NUMBER($year, useGrouping: "false") }
# Whole local time. $hour-cycle chooses a convention; $hour is 0–23,
# $hour-twelve is 1–12, $minute is 0–59, $period is am/pm; $zone is a label.
formats-time = { $hour-cycle ->
    [h12] { NUMBER($hour-twelve, useGrouping: "false") }:{ NUMBER($minute, minimumIntegerDigits: 2, useGrouping: "false") } { $period ->
        [am] AM
       *[pm] PM
        } { $zone }
   *[h24] { NUMBER($hour, minimumIntegerDigits: 2, useGrouping: "false") }:{ NUMBER($minute, minimumIntegerDigits: 2, useGrouping: "false") } { $zone }
    }
# Whole local date and time, with the same typed parts as the two patterns.
formats-date-time = { formats-date } at { formats-time }
