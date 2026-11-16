## Host-local workspace labels. Session metadata is a separate domain.
# Label manager title text; shown inside the adaptive dialog.
labels-title = Labels
# Label manager manage text; shown inside the adaptive dialog.
labels-manage = Manage labels
# Header for a retained name/color draft, including pending save/inspection.
labels-edit = Edit label
# Destructive confirmation target; $name is the original host label, never the unsaved replacement.
labels-delete-title = Delete “{ $name }”?
# Label manager search text; shown inside the adaptive dialog.
labels-search = Search labels
# Label manager name text; shown inside the adaptive dialog.
labels-name = Label name
# Label manager edit view: the name field's label (Paseo's "Name").
labels-field-name = Name
# Label manager edit view: the colour swatch row's label (Paseo's "Color").
labels-field-color = Color
# Label manager create text; shown inside the adaptive dialog.
labels-create = Create label
# Label manager save text; shown inside the adaptive dialog.
labels-save = Save
# Label manager delete text; shown inside the adaptive dialog.
labels-delete = Delete
# Label manager cancel text; shown inside the adaptive dialog.
labels-cancel = Cancel
# Label manager back text; shown inside the adaptive dialog.
labels-back = Back
# Label manager host text; shown inside the adaptive dialog.
labels-host = Host
# Label manager empty line, for no labels and for no match alike (Paseo's `manage.empty`).
labels-empty = No labels on this host.
# Label manager no matches text; shown inside the adaptive dialog.
labels-no-matches = No matching labels
# Label manager no hosts text; shown inside the adaptive dialog.
labels-no-hosts = Add a host to manage labels.
# A book alias has not yet answered with a verified node identity.
labels-connect = Connect to a host to load its labels.
# Label manager loading text; shown inside the adaptive dialog.
labels-loading = Loading labels…
# Label manager offline text; shown inside the adaptive dialog.
labels-offline = This host is offline.
# Label manager unsupported text; shown inside the adaptive dialog.
labels-unsupported = Update this host to manage labels.
# Label manager load failed text; shown inside the adaptive dialog.
labels-load-failed = Could not load labels. Try again.
# Label manager pending text; shown inside the adaptive dialog.
labels-pending = Updating labels…
# Label manager inspecting text; shown inside the adaptive dialog.
labels-inspecting = Checking affected workspaces…
# Label manager confirm text; shown inside the adaptive dialog.
labels-confirm = Delete this label and remove it from affected workspaces?
# Advisory deletion inspection count; $count is the numeric workspace count.
labels-affected = { NUMBER($count) ->
    [one] This label is used by one workspace.
   *[other] This label is used by { NUMBER($count) } workspaces.
    }
# Label manager taken text; shown inside the adaptive dialog.
labels-taken = A label already uses this name.
# Label manager missing text; shown inside the adaptive dialog.
labels-missing = This label no longer exists.
# Label manager failed text; shown inside the adaptive dialog.
labels-failed = Could not update labels. Try again.
# Label manager invalid text; shown inside the adaptive dialog.
labels-invalid = Enter a label name.
# Name of the violet swatch in the label editor; keep short.
labels-color-violet = Violet
# Name of the sky swatch in the label editor; keep short.
labels-color-sky = Sky
# Name of the emerald swatch in the label editor; keep short.
labels-color-emerald = Emerald
# Name of the orange swatch in the label editor; keep short.
labels-color-orange = Orange
# Name of the pink swatch in the label editor; keep short.
labels-color-pink = Pink
# Name of the indigo swatch in the label editor; keep short.
labels-color-indigo = Indigo
# Name of the teal swatch in the label editor; keep short.
labels-color-teal = Teal
# Name of the red swatch in the label editor; keep short.
labels-color-red = Red
# Name of the amber swatch in the label editor; keep short.
labels-color-amber = Amber
# Name of the blue swatch in the label editor; keep short.
labels-color-blue = Blue

# Exact selected connection retired; never replay the old request on another admission.
labels-changed = The selected connection changed. Try again.

# Canonical workspace caller marks the registry target inactive or read-only; connecting cannot unlock it.
labels-read-only = Labels cannot be changed in this workspace.
