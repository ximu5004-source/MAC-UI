import { mount } from "svelte";
import Preview from "./Preview.svelte";
import "libs/ui/svelte/styles/mac-panels.css";
import "libs/ui/styles/glass-material.css";
import "src/ui/svelte/quick-settings/control-center.css";
import "src/ui/svelte/flyouts/frosted-flyouts.css";
mount(Preview, { target: document.getElementById("root")! });
