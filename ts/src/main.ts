import { Object as JavaObject } from "@Java/java/lang" with { type: "java" };
import { Teste } from "./classes/Teste";

declare global {
  var mensagem: string;
  var contexto: any;
  export interface Object extends JavaObject {
    isObject(obj: any): boolean;
  }
}

Object.isJavaObject = function (obj: Object) {
  return obj.getClass !== undefined;
};

const test = new Teste();

test.doAction(contexto);
