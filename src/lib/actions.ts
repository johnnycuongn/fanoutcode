// LiveSpec's menu action ids, kept only because the unchanged `crates/livespec-core` checks them against its menu in
// `menu::tests::every_item_is_a_frontend_action_and_every_action_has_an_item`, which reads this exact path.
// Fanout Code's own keyboard map lives elsewhere (Plan 06); nothing in the webview imports this type.
export type ActionId =
  | "new-tab"
  | "new-window"
  | "open-project"
  | "reopen-tab"
  | "close-tab"
  | "close-window"
  | "next-tab"
  | "prev-tab"
  | "toggle-sidebar"
  | "settings";
