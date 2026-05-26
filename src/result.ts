import { mount } from "svelte";
import Result from "./Result.svelte";
import "./lib/styles.css";

const app = mount(Result, {
  target: document.getElementById("app")!,
});

export default app;
