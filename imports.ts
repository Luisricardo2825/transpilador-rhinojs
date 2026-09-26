import * as fs from "fs";
import * as path from "path";

const BASE_FOLDER = "C:/Users/luis_/repositorios/typescript/Packages";

export interface PackageInfo {
  group: string;
  classes: ClassInfo[];
}
export interface ClassInfo {
  name: string;
  simpleName: string;
  extends: string;
  implements: string[];
  constructors: Constructor[];
  methods: Method[];
  fields: Field[];
  isEnum: boolean;
  isInterface: boolean;
}

export interface Constructor {
  name: string;
  params: string[];
}
export interface Field {
  name: string;
  type: string;
  isStatic: boolean;
}
export interface Method {
  name: string;
  params: string[];
  returnType: string;
  isStatic: boolean;
}
function generateTypes(JDK_JSON: string, PACKAGE_NAME: string) {
  const typeMap: Record<string, string> = {
    int: "number",
    long: "number",
    double: "number",
    float: "number",
    boolean: "boolean",
    char: "string",
    byte: "number",
    short: "number",
    "char[]": "string[] | null | undefined",
    "java.lang.String": "string | null | undefined",
    "java.lang.Integer": "number | Integer | null | undefined",
    "java.lang.Long": "number | Long | null | undefined",
    "java.lang.Double": "number | Double | null | undefined",
    "java.lang.Float": "number | Float | null | undefined",
    "java.lang.Boolean": "boolean | Boolean | null | undefined",
    "java.lang.Character": "string | Character | null | undefined",
    "java.util.List": "any[] | List",
    "java.util.Map": "Record<string, any> | Map",
    "java.lang.Object": "Object | null | undefined",
  };

  let uniqueClasses: ClassInfo[] = [];
  let pathImports: Record<string, string> = getClassPaths("./classes.json");
  let names: string[] = Object.keys(pathImports);
  let current: ClassInfo | undefined;

  function getClassPaths(jsonPath: string) {
    if (!fs.existsSync(jsonPath)) {
      return {};
    }
    const classes: Record<string, string> = JSON.parse(
      fs.readFileSync(jsonPath, "utf8")
    );
    return classes;
  }
  function savePathImports(jsonPath: string) {
    fs.writeFileSync(jsonPath, JSON.stringify(pathImports, null, 2));
  }
  function mapType(javaType: string, imports: Set<string>): string {
    if (names.includes(javaType)) {
      imports.add(toImport(javaType) || "");
    }

    if (javaType.endsWith("[]"))
      return `(${mapType(javaType.slice(0, -2), imports)})[]`;
    return typeMap[javaType] || javaType.split(".").pop() || "any";
  }

  function generateParams(params: string[], imports: Set<string>) {
    return params.map((p, i) => `p${i}: ${mapType(p, imports)}`).join(", ");
  }

  function toImport(cls: string) {
    const simpleName = cls.split(".").pop() || "any";
    if (cls === current?.name) return null;

    const currentFile = pathImports[current!.name];
    const targetFile = pathImports[cls];
    if (!currentFile || !targetFile) return null;

    let rel = path
      .relative(path.dirname(currentFile), targetFile)
      .replace(/\.d\.ts$/, "");
    if (!rel.startsWith(".")) rel = "./" + rel;
    rel = rel.split(path.sep).join("/");

    return `import type { ${simpleName} } from "${rel}";`;
  }

  function resolveModifiers(m: Method) {
    return `  ${m.isStatic ? "static " : ""}`;
  }
  function resolveExtends(cls: ClassInfo, imports: Set<string>) {
    const simpleName = cls.extends.split(".").pop() || "any";
    imports.add(toImport(cls.extends) || "");
    return `extends ${simpleName}`;
  }
  function resolveImplements(cls: ClassInfo, imports: Set<string>) {
    const parents = cls.implements
      .map((i) => names.find((c) => c === i))
      .filter(Boolean);
    parents.forEach((p) => imports.add(toImport(p!) || ""));
    return parents.length
      ? `implements ${parents
          .map((p) => p!.split(".").pop() || "any")
          .join(", ")} `
      : "";
  }

  function pathForClass(cls: ClassInfo) {
    const folders = cls.name.split(".");
    const fileName = folders.pop();
    const dir = path.join(BASE_FOLDER, folders.join("/"));
    fs.mkdirSync(dir, { recursive: true });
    pathImports[cls.name] = path.join(dir, `${fileName}.d.ts`);
  }

  function createTypes() {
    const packageInfo: PackageInfo = JSON.parse(
      fs.readFileSync(JDK_JSON, "utf8")
    );
    const classes: ClassInfo[] = packageInfo.classes;
    uniqueClasses = Array.from(
      new Map(classes.map((c) => [c.name, c])).values()
    );
    names = [
      ...names,
      ...Array.from(new Set(uniqueClasses.map((c) => c.name))),
    ];

    for (const cls of uniqueClasses) pathForClass(cls);

    for (const cls of uniqueClasses) {
      current = cls;
      const imports: Set<string> = new Set();

      let content = `// ${cls.name}\n`;
      content += `export ${cls.isInterface ? "interface" : "class"} ${
        cls.simpleName
      } ${resolveExtends(cls, imports)} ${resolveImplements(cls, imports)} {\n`;

      if (!cls.isInterface)
        cls.constructors.forEach(
          (c) =>
            (content += `  constructor(${generateParams(
              c.params,
              imports
            )});\n`)
        );
      cls.methods.forEach(
        (m) =>
          (content += `${resolveModifiers(m)}${m.name}(${generateParams(
            m.params,
            imports
          )}): ${mapType(m.returnType, imports)};\n`)
      );
      cls.fields.forEach(
        (f) =>
          (content += `${f.isStatic ? "static " : ""}${f.name}: ${mapType(
            f.type,
            imports
          )};\n`)
      );
      content += `}\nexport default ${cls.simpleName};`;

      content = Array.from(imports).filter(Boolean).join("\n") + "\n" + content;
      fs.writeFileSync(pathImports[cls.name], content);
    }
  }

  function createGlobalForPackage(packagePath: string) {
    const files = fs.readdirSync(packagePath);
    let content = "";
    files.forEach((file) => {
      if (fs.lstatSync(path.join(packagePath, file)).isDirectory()) {
        return createGlobalForPackage(path.join(packagePath, file));
      }
      const name = path.parse(file).name.replace(".d", "");
      // Remove ".d"

      if (name === "index") return;
      content += `export { ${name} } from "./${name}";\n`;
    });
    fs.writeFileSync(path.join(packagePath, "index.d.ts"), content);
  }

  function createPackagesDTS() {
    let content = `declare namespace ${PACKAGE_NAME} {\n`;
    const javaDir = path.join(BASE_FOLDER, PACKAGE_NAME);
    fs.readdirSync(javaDir)
      .filter((f) => fs.lstatSync(path.join(javaDir, f)).isDirectory())
      .forEach((pkg) => {
        content += `    const ${pkg}: typeof import("./java/${pkg}/index");\n`;
      });
    content += "  }\n";
    fs.writeFileSync(path.join(BASE_FOLDER, "java.d.ts"), content);
  }

  // === Execução ===
  createTypes();

  // gera globals por pacote
  const javaDir = path.join(BASE_FOLDER, PACKAGE_NAME);

  fs.readdirSync(javaDir)
    .filter((f) => fs.lstatSync(path.join(javaDir, f)).isDirectory())
    .forEach((pkg) => {
      createGlobalForPackage(path.join(javaDir, pkg));
    });
  // generate index.d.ts for pkg
  createGlobalForPackage(javaDir);

  // createPackagesDTS();
  savePathImports("./classes.json");
  console.log("✅ Tipos e namespaces gerados com sucesso!");
}

// get all jsons from "./json"
const jsons = fs
  .readdirSync("./json")
  .filter((f) => f.endsWith(".json"))
  .map((f) => path.join("./json", f));

fs.rmSync(BASE_FOLDER, { recursive: true, force: true });

jsons.forEach((json) => {
  // if (!json.endsWith("sanws.json")) return;
  const jdk: PackageInfo = JSON.parse(fs.readFileSync(json, "utf8"));
  generateTypes(json, jdk.group.replaceAll(".", "/"));
});
