# 模型分类优先级经验

更新日期：2026-05-19

## 背景

模型列表接口新增了 `category` 字段，但它不是必填项。同时，很多图片模型也会暴露 `openai-video`、`video-generation` 之类的异步或兼容端点。

如果仍然只按 `supported_endpoint_types` 或 `id` 猜类型，就很容易把图片模型误判成视频模型。

## 问题表现

- `gpt-4o-image-async`、`gpt-image-*` 这类图片模型，因为带了 `openai-video`，被错分到视频。
- 旧的静态模型配置命中后，运行时接口返回的 `category` 没有机会修正类型。
- `category` 出现 `生图`、`文本`、`研究` 这类业务值时，如果不做归一化，分类会漂。

## 修复思路

- `category` 存在时优先使用。
- 归一化常见值：
  - `生图`、`图片`、`图像`、`绘图`、`image` -> `image`
  - `视频`、`video` -> `video`
  - `文本`、`研究`、`chat`、`text` -> `text`
  - `音频`、`音乐`、`voice`、`audio` -> `audio`
- `category` 缺失时，再回退到 `id`、endpoint hint、vendor 的旧推断逻辑。
- 只要 `id` 命中 `image`，就应先保住图片归类，不要被视频端点抢走。
- 静态配置如果被接口 `category` 明确覆盖，要同步清理不匹配的默认参数，避免图片模型挂着视频默认值。

## 经验规则

- 有 `category`，先信 `category`。
- 有 `image`，一定按图片模型处理。
- 图片模型同时支持视频端点，不等于视频模型。
- `category` 是分类信号，不是装饰字段；接口给了就该进判定链路。
- 这种修复优先改判定顺序，不要靠新增更多特例堆规则。

## 验证

- `node_modules/.bin/vitest run packages/drawnix/src/utils/__tests__/runtime-model-discovery.test.ts`
- `git diff --check`
- 用真实模型列表抽样确认：`生图 + openai-video` 仍归图片。
