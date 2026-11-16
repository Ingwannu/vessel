# Readonly recall fixture result binding

The exact f36c GPUI recall regression passed1/0, but its mounted-tab setup discarded the returned TabId and emitted unused_results under the workspace warning policy. Bind that setup result once; retain the same real layout call, target, unwrap and every text/selection/recall assertion. Product behavior and the passed test scenario are unchanged. This closes an observed fixture lint without treating the old runtime result as current receiving-union strict proof.
