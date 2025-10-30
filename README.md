# cvr-i18

一个用于检查和整理国际化 JSON 文件的命令行工具。

## 功能

- **检查重复键**: 检测 JSON 文件中的重复顶级键。
- **检查缺少键**: 相对于基准文件（默认 en.json）检查其他文件缺少的键。
- **导出缺少键**: 将缺少的键导出到指定目录的 JSON 文件中。
- **排序键**: 根据基准文件的键顺序重新排列其他文件的键。

## 安装

```bash
cargo install --path .
```

## 使用

### 默认行为

运行工具时不带参数会显示帮助信息。

```bash
cvr-i18
```

### 指定目录

工具会自动检测默认目录：`./locales` 或 `./src/locales`。如果需要指定其他目录，使用 `-d` 参数。

```bash
cvr-i18 -d /path/to/locales
```

### 检查重复键

检查目录中所有 JSON 文件的重复顶级键。

```bash
cvr-i18 -k
```

### 检查缺少键

相对于 `en.json` 检查其他文件缺少的键。

```bash
cvr-i18 -m
```

### 导出缺少键

将缺少的键导出到指定目录。

```bash
cvr-i18 -m -e ./exports
```

### 排序键

根据基准文件的键顺序重新排列其他文件的键。

```bash
cvr-i18 -s
```

### 指定基准文件

使用 `-b` 指定基准文件（默认为 `en.json`）。

```bash
cvr-i18 -s -b base.json
```

## 参数说明

- `-d, --directory <DIR>`: 指定包含 JSON 文件的目录。默认为 `./locales` 或 `./src/locales`。
- `-k, --duplicated-key`: 检查重复的顶级键。
- `-m, --missing-key`: 检查相对于基准文件的缺少键。
- `-e, --export <DIR>`: 导出缺少键到指定目录。
- `-s, --sort`: 排序键顺序。
- `-b, --base <FILE>`: 指定基准文件，默认为 `en.json`。

## 示例

1. 检查 `./locales` 目录中的重复键：

   ```bash
   cvr-i18 -k
   ```

2. 检查缺少键并导出：

   ```bash
   cvr-i18 -m -e ./missing_keys
   ```

3. 排序键：

   ```bash
   cvr-i18 -s
   ```

## 退出码

- `0`: 成功，无问题。
- `1`: 发现问题（如重复键或缺少键）。
- `2`: 错误（如文件不存在、解析失败）。

## 许可证

[GPLv3 License](LICENSE)
