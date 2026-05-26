import { mount } from "svelte";
import Settings from "./Settings.svelte";
import "./lib/styles.css";

const app = mount(Settings, {
  target: document.getElementById("app")!,
});

export default app;
