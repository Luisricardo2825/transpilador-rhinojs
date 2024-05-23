// Default imports

const System = java.lang.System;


const Files = java.nio.file.Files;
const Path = java.nio.file.Path;
const Paths = java.nio.file.Paths;
const Arrays = java.util.Arrays;
const JsonObject = Packages.com.google.gson.JsonObject;
const Collections = java.util.Collections;
const List = java.util.List;
const Collectors = java.util.stream.Collectors;
const ServiceContext = Packages.br.com.sankhya.ws.ServiceContext;
const JString = java.lang.String;
var values = new JsonObject();
var res = new JsonObject();
var path = java.lang.System.getProperty("org.jboss.boot.log.file");
var filePath = Paths.get(path);
values.addProperty("filePath", filePath.toString());
var bytes = Files.readAllBytes(filePath);
var lines = new JString(bytes).split(System.lineSeparator());
list = Arrays.asList(lines);
Collections.reverse(list);
list = list.stream().limit(1000).collect(Collectors.toList());
var content = JString.join(System.lineSeparator(), list);
values.addProperty("content", content);
res.addProperty("log", content);
ServiceContext.getCurrent().setJsonResponse(res);
mensagem = content;