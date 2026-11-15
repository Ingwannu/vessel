## Safe preference-change refusal notices, shared by Settings pages.

# Whole refusal for the shortcut-action bound. $limit is MAX_ACTIONS, a number.
settings-notices-actions = { $limit ->
    [one] That change was not kept. Keys can be kept for at most { NUMBER($limit) } action.
   *[other] That change was not kept. Keys can be kept for at most { NUMBER($limit) } actions.
    }

# Whole refusal for the per-machine defaults bound. $limit is MAX_MACHINES, a number.
settings-notices-machines = { $limit ->
    [one] That change was not kept. Defaults can be kept for at most { NUMBER($limit) } machine.
   *[other] That change was not kept. Defaults can be kept for at most { NUMBER($limit) } machines.
    }

# Whole safe fallback if the bounded explanation could not be prepared.
settings-notices-change-not-kept = That change was not kept.
