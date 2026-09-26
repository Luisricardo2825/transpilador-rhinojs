import {
  AcaoRotinaJava,
  ContextoAcao,
} from "@Java/br/com/sankhya/extensions/actionbutton";
import { System } from "@Java/java/lang";

export class Teste implements AcaoRotinaJava {
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
