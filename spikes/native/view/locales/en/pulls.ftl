## A session's pull request: the git menu's entry, the Create PR form and
## the facts. Reference: Paseo i18n/resources/{en,ko}.ts:
## workspace.git.actions (createPr, viewPr, unavailable.createPrNoCommits,
## unavailable.createPrNoForge, toasts.failedCreatePr), workspace.git.pr
## (states, sections, checksOverview) and panels.pullRequest.
## Paseo says PR in actions and pull request in panels; so does Vessel.
## $host is a forge's host name, such as github.com.

# Menu entry and the form's send button. Keep short.
pulls-create = Create PR
# The entry while the request runs. Keep short.
pulls-creating = Creating PR...
# Toast after the pull request was made. Keep short.
pulls-created = PR Created
# Toast when one was already open for the branch. Keep short.
pulls-existing = A PR for this branch was already open
# Menu entry and the git row's link tooltip action. Keep short.
pulls-view = View PR
# Name of the thing, on the git row's link tooltip. Keep short.
pulls-label = Pull request
# Toast title when making it failed. Keep short.
pulls-failed = Failed to create PR

# Why Create PR is unavailable: nothing new on the branch.
pulls-no-commits = Create PR isn't available because this branch doesn't have any new commits yet
# Why Create PR is unavailable: a detached HEAD.
pulls-no-branch = Create PR isn't available because you aren't on a branch
# Why Create PR is unavailable: the branch has no remote, or its remote is a
# path or a host that is not a forge (the node says the same for both).
pulls-no-remote = Create PR isn't available because this branch has no GitHub remote
# Why Create PR is unavailable: the machine reads no forge at $host.
pulls-no-forge = Create PR isn't available right now because { $host } isn't connected
# Why Create PR is unavailable: the machine is not signed in to $host.
pulls-signed-out = Create PR isn't available because this machine isn't signed in to { $host }. Run gh auth login on the machine.
# Why Create PR is unavailable: $host refused the machine's sign-in.
pulls-refused = Create PR isn't available because { $host } refused this machine's sign-in
# Why Create PR is unavailable: $host's rate limit.
pulls-rate-limited = Create PR isn't available right now because { $host } is limiting requests. Try again later.
# Why Create PR is unavailable: $host can't be reached.
pulls-unreachable = Create PR isn't available right now because { $host } can't be reached
# Why Create PR is unavailable: the machine watches its maximum.
pulls-limit = Create PR isn't available because this machine is watching as many pull requests as it can
# Why Create PR is unavailable: the session ended.
pulls-ended = Create PR isn't available because this session has ended
# Why Create PR is unavailable, for a reason this app does not know.
pulls-unavailable = Create PR isn't available right now

# The form's title. Keep short.
pulls-form-title = Create pull request
# Title field label. Keep short.
pulls-field-title = Title
# Description field label. Keep short.
pulls-field-body = Description
# Base branch field label. Keep short.
pulls-field-base = Base branch
# The base choice that leaves it to the machine. Keep short.
pulls-default-base = Default branch
# Draft check box label. Keep short.
pulls-field-draft = Create as draft
# Close the form without sending. Keep short.
pulls-cancel = Cancel
# The title is empty.
pulls-title-blank = Enter a title
# The title is too long.
pulls-title-long = Keep the title to 256 characters
# The description is too long.
pulls-body-long = Keep the description under 64 KB
# The wire refused the title or description for another rule.
pulls-not-accepted = This title or description can't be sent

# Refused: an agent's branch is not on the remote.
pulls-not-pushed = Push this branch before creating its PR
# Refused: pushing the branch first was refused; the git reason follows.
pulls-push-refused = Pushing this branch was refused
# Refused by the forge; its own message is shown under this.
pulls-rejected = The pull request was refused
# Refused for a reason this app does not know.
pulls-refused-unknown = The pull request wasn't created

# A pull request's state, beside its icon. Keep short.
pulls-state-open = Open
# A draft's state. Keep short.
pulls-state-draft = Draft
# A merged one's state. Keep short.
pulls-state-merged = Merged
# A closed one's state. Keep short.
pulls-state-closed = Closed
# A state this app does not know. Keep short.
pulls-state-unknown = Unknown

# Section title. Keep short.
pulls-checks = Checks
# Section title. Keep short.
pulls-activity = Activity
# Checks headline: one failed.
pulls-checks-failure = Some checks were not successful
# Checks headline: some still run.
pulls-checks-pending = Some checks haven't completed yet
# Checks headline: all passed.
pulls-checks-success = All checks have passed
# Checks headline: none reported.
pulls-checks-none = No checks
# Checks headline: not read yet.
pulls-checks-unknown = Checks haven't been read yet
# A count of failing checks. $count is a number.
pulls-count-failing = { NUMBER($count) } failing
# A count of running checks. $count is a number.
pulls-count-pending = { NUMBER($count) } in progress
# A count of passed checks. $count is a number.
pulls-count-passed = { NUMBER($count) } successful
# More failing checks than listed.
pulls-checks-more = More failing checks aren't listed

# Review verdict. Keep short.
pulls-approved = Approved
# Review verdict. Keep short.
pulls-changes-requested = Changes requested
# Review verdict. Keep short.
pulls-review-required = Review required
# Unresolved review threads. $count is a number.
pulls-unresolved = { NUMBER($count) ->
    [one] { $count } unresolved thread
   *[other] { $count } unresolved threads
    }

# Mergeability. Keep short.
pulls-merge-ready = Ready to merge
# Mergeability. Keep short.
pulls-merge-conflicts = Conflicts with its base
# Mergeability. Keep short.
pulls-merge-blocked = Blocked by a rule
# Mergeability. Keep short.
pulls-merge-unstable = Checks aren't all passing
# Mergeability. Keep short.
pulls-merge-behind = Behind its base
# Mergeability not computed yet. Keep short.
pulls-merge-unknown = Not known yet

# Open the page on the forge. $host names it. Keep short.
pulls-open-on = Open on { $host }
# Read it from the forge now. Keep short.
pulls-refresh = Refresh
# Empty view title.
pulls-empty-title = No pull request yet
# Empty view description.
pulls-empty-description = Create a pull request for this checkout to see its details here.

# A card's checks outcome after its glyph. Keep to one word.
pulls-card-passed = passed
# A card's checks outcome after its glyph. Keep to one word.
pulls-card-failed = failed
# A card's checks outcome after its glyph. Keep to one or two words.
pulls-card-running = running

# Menu entry: merge with a squash commit. Keep short.
pulls-merge-squash = Merge PR (squash)
# Menu entry: merge with a merge commit. Keep short.
pulls-merge-merge = Merge PR (merge)
# Menu entry: merge by rebasing. Keep short.
pulls-merge-rebase = Merge PR (rebase)
# A merge entry while the merge runs. Keep short.
pulls-merging = Merging PR...
# Toast after the merge. Keep short.
pulls-merged = PR merged
# Toast title when the merge failed. Keep short.
pulls-merge-failed = Failed to merge PR
# Merge refused: the session has no pull request.
pulls-merge-missing = Merge PR isn't available because there isn't a pull request yet
# Merge refused: the session's pull request changed. $number is its number, as text.
pulls-merge-other = Merge PR isn't available because this session's pull request is now #{ $number }
# Merge refused: already merged.
pulls-merge-already = Merge PR isn't available because the pull request is already merged
# Merge refused: closed.
pulls-merge-closed = Merge PR isn't available because the pull request is closed
# Merge refused: a draft.
pulls-merge-draft = Merge PR isn't available because the pull request is still a draft
# Merge refused: the branch moved since the person looked.
pulls-merge-moved = Merge PR isn't available because the branch has new commits since you looked. Try again after the next refresh.
# Merge refused: the forge does not say it is ready.
pulls-merge-not-ready = Merge PR isn't available until GitHub reports the PR is ready to merge
# Merge refused: the repository does not allow the method.
pulls-merge-method = Merge PR isn't available because this repository doesn't allow this merge method
# Merge refused: the repository uses a merge queue.
pulls-merge-queue = Merge PR isn't available here because this repository uses a merge queue
# Merge refused: the forge cannot merge exactly the commits seen with this method.
pulls-merge-pin = Merge PR isn't available because GitHub can't merge exactly the commits you saw with this method
# Merge refused: the machine cannot reach the forge.
pulls-merge-no-github = Merge PR isn't available right now because GitHub isn't connected
# Merge refused by the forge; its own message is shown under this.
pulls-merge-rejected = The merge was refused
# Merge refused for a reason this app does not know.
pulls-merge-refused-unknown = The pull request wasn't merged
# Switch label in the pull request view. Keep it one line.
pulls-feedback = Send failing checks and reviews to the session
