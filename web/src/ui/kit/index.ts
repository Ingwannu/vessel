// The kit (ADR 0021, DESIGN.md §6.13): every control the screens use.
// Screens import from here, never from `@base-ui/react`, so behaviour and
// look stay in one place.

export { Avatar } from "./avatar";
export { Badge, type BadgeLook, type Tone } from "./badge";
export { Banner } from "./banner";
export {
  Button,
  type ButtonProps,
  type ButtonShape,
  type ButtonSize,
  type ButtonVariant,
  IconButton,
} from "./button";
export { Checkbox } from "./checkbox";
export { Collapsible } from "./collapsible";
export { type Choice } from "./choice";
export { Combobox, type Suggestion } from "./combobox";
export { CommandLine } from "./command-line";
export { type Command, CommandPalette } from "./command";
export { Dialog, type DialogSize } from "./dialog";
export { EmptyState } from "./empty-state";
export { Field, FieldGroup } from "./field";
export { Icon, type IconName, type IconSize } from "./icon";
export { Kbd } from "./kbd";
export {
  Menu,
  MenuCheckboxItem,
  MenuGroup,
  MenuItem,
  MenuRadioGroup,
  MenuSeparator,
  MenuSubmenu,
} from "./menu";
export { MenuNote, MenuSwitchItem } from "./menu-switch";
export { Popover } from "./popover";
export { Row, type RowDetail, RowList, type RowMark } from "./row";
export { Segmented, type SegmentedOptions } from "./segmented";
export { Select } from "./select";
export { Skeleton } from "./skeleton";
export { Spinner } from "./spinner";
export { StatusDot } from "./status-dot";
export { Switch } from "./switch";
export { type Tab, Tabs } from "./tabs";
export { TextArea } from "./text-area";
export { TextField } from "./text-field";
export { Tooltip, TooltipProvider } from "./tooltip";
