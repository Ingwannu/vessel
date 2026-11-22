# Page settings sidebar icons

TRACKER 843 follows Paseo's `packages/app/src/screens/settings-screen.tsx:165–230`.
General uses Settings, Appearance uses Palette and About uses Info.
Shortcuts, Notifications and Diagnostics already use Keyboard, Bell and
Stethoscope. The app Terminal and Files sections use SquareTerminal and
Files. Paseo uses MessageSquare for Chat. The page's Chat section belongs
to F-L74. Vessel's machine Terminals page manages running terminals.
The sidebar footer also uses Settings, as in Paseo's
`packages/app/src/components/left-sidebar.tsx:447–452`. The Machines
filter keeps the sliders mark.

The sidebar and the command palette share `sectionIcon` so both show
the same mark for a settings destination. The page kit already applies
mask tint and sizing to decorative icons. The two new masks preserve
the original Settings and Palette geometry from `lucide-react-native`
1.50.0, pinned in Paseo's package lock, and use the kit's stroke width.
The existing Info mask serves About. The
[Lucide license](../design/brand/lucide-LICENSE.txt) matches the notice on
the native package line.
