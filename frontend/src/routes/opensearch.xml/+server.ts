export function GET() {
	return new Response(
		`<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
    <ShortName>Otter</ShortName>
    <LongName>Otter - Hack Club Projects Search</LongName>
    <Tags>hackclub projects search</Tags>
    <Description>Search engine for all Hack Club projects</Description>
    <Url type="text/html" template="https://search.shymike.dev/?q={searchTerms}" />
    <Image height="64" width="64" type="image/png">https://search.shymike.dev/favicon.png</Image>
    <Language>en-us</Language>
    <OutputEncoding>UTF-8</OutputEncoding>
    <InputEncoding>UTF-8</InputEncoding>
</OpenSearchDescription>`,
		{
			headers: {
				'Content-Type': 'application/opensearchdescription+xml'
			}
		}
	);
}
