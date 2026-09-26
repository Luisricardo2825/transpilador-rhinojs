import { System } from "@Java/java/lang";

class JavaEntity {
  static [Symbol.hasInstance](value: unknown) {
    return Object.isJavaObject(value);
  }
}

const javaLike = {
  getClass() {
    return "br.com.sankhya.Example";
  },
};

const plainObject = {};

const javaLikeMatches = javaLike instanceof JavaEntity;
const plainObjectMatches = plainObject instanceof JavaEntity;

mensagem =
  "javaLike: " +
  (javaLikeMatches ? "Sim" : "Não") +
  "; plainObject: " +
  (plainObjectMatches ? "Sim" : "Não");

System.out.println(mensagem);
