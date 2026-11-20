// What a project page's action that failed says (TRACKER 820): what did not
// happen as the Alert's title, the node's reason as its description, as
// Paseo's project settings and setup panel word their failures ("Couldn't
// save paseo.json" over the reason).

export interface ActionFailure {
  /** What did not happen ("Setup did not start"). */
  title: string;
  /** Why, as the node said it. */
  message: string;
}
