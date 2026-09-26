import { System } from "@Java/java/lang";
import { Object as JavaObject } from "@Java/java/lang";

import {
  AcaoRotinaJava,
  ContextoAcao,
} from "@Java/br/com/sankhya/extensions/actionbutton";

declare global {
  var mensagem: string;
  var contexto: any;
  export interface Object extends JavaObject {
    isObject(obj: any): boolean;
  }
}

Object.isJavaObject = function (obj: any) {
  return obj.getClass !== undefined;
};
class Teste implements AcaoRotinaJava {
  doAction(ctx: ContextoAcao): void {
    const linhas = ctx.getLinhas();

    let msg = "";
    for (const linha of linhas) {
      System.out.println(linha.getCampo("CODJAR"));
      msg += linha.getCampo("CODJAR") + " ";
    }
    // ctx.setMensagemRetorno(msg);
    mensagem = `Mensagem: ${msg}\n é obejto java? ${
      Object.isJavaObject(ctx) ? "Sim" : "Não"
    }`;
  }
}

const test = new Teste();

test.doAction(contexto);
