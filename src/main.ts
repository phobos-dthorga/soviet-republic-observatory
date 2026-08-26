import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { initializeLanguage } from "./lib/i18n/service";

const target = document.getElementById("app");

if (!target) throw new Error("Application root is unavailable");

initializeLanguage();
mount(App, { target });
