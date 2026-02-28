import type { DxPluginApi } from "dx/plugin-sdk";
import { emptyPluginConfigSchema } from "dx/plugin-sdk";
import { feishuPlugin } from "./src/channel.js";

const plugin = {
  id: "feishu",
  name: "Feishu",
  description: "Feishu (Lark) channel plugin",
  configSchema: emptyPluginConfigSchema(),
  register(api: DxPluginApi) {
    api.registerChannel({ plugin: feishuPlugin });
  },
};

export default plugin;
