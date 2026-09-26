import { System } from "@Java/java/lang";
import { Teste } from "./classes/Teste";

const obj: Record<string | symbol, any> = {};

obj["teste"] = "deu certo";
const symbol = Symbol("syn");
obj[symbol] = "simbolo";
obj[symbol] += "simbolo";

const obj2: Record<string | symbol, any> = {};

const isJava = Object["isJavaObject"](obj2);
obj2["teste"] = "deu certo";
const symbol2 = Symbol("syn");
obj2[symbol2] = "simbolo";
obj2[symbol2] += "simbolo";
const a = new Teste();

if (a instanceof Teste) {
  System.out.println("é instancia");
}
const ehJava = isJava ? "Sim" : "Não";

mensagem = "é objeto java?" + ehJava;
